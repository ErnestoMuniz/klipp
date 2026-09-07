use std::sync::{Arc, OnceLock};

use ashpd::desktop::global_shortcuts::GlobalShortcuts;
use ashpd::desktop::Session;
use futures_util::lock::Mutex as AsyncMutex;

use crate::core::i18n::{t, t_fmt};
use crate::core::state::Shared;

pub const OVERLAY_ID: &str = "klipp-overlay";

/// Proxy + sessão do portal, compartilhados entre o loop de escuta e o
/// rebind. As streams de sinal são owned (`use<>`), então o lock só é
/// segurado para criar streams e para (re)bindar.
struct Service {
    shortcuts: GlobalShortcuts,
    session: Session<GlobalShortcuts>,
}

static SERVICE: OnceLock<Arc<AsyncMutex<Service>>> = OnceLock::new();

async fn ensure_service() -> anyhow::Result<Arc<AsyncMutex<Service>>> {
    if let Some(svc) = SERVICE.get() {
        return Ok(svc.clone());
    }
    use ashpd::desktop::CreateSessionOptions;
    let shortcuts = GlobalShortcuts::new().await?;
    let session = shortcuts
        .create_session(CreateSessionOptions::default())
        .await?;
    let svc = Arc::new(AsyncMutex::new(Service {
        shortcuts,
        session,
    }));
    let _ = SERVICE.set(svc.clone());
    Ok(SERVICE.get().unwrap().clone())
}

fn spec_trigger(preferred: &str) -> String {
    // Portal segue a spec freedesktop/shortcuts: `CTRL+ALT+s`, sem colchetes.
    // `[Alt+Shift+S]` quebra o parse (para no `[`) e o bind fica vazio —
    // overlay nunca dispara. Normaliza modificadores para maiúsculo.
    let clean = preferred.trim().trim_start_matches('[').trim_end_matches(']');
    let mut mods = Vec::new();
    let mut key = String::new();
    for part in clean.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()) {
        match part.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods.push("CTRL"),
            "ALT" => mods.push("ALT"),
            "SHIFT" => mods.push("SHIFT"),
            "LOGO" | "SUPER" | "META" | "WIN" => mods.push("LOGO"),
            "NUM" => mods.push("NUM"),
            _ => key = part.to_string(),
        }
    }
    if key.is_empty() {
        key = "S".to_string();
    }
    if mods.is_empty() {
        key
    } else {
        format!("{}+{key}", mods.join("+"))
    }
}

async fn bind(
    svc: &Arc<AsyncMutex<Service>>,
    preferred: &str,
    lang: &str,
) -> anyhow::Result<Option<String>> {
    use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, NewShortcut};
    let trigger = spec_trigger(preferred);
    let desc = t(lang, "shortcut.portal_desc");
    let new_shortcut =
        NewShortcut::new(OVERLAY_ID, desc.as_str()).preferred_trigger(Some(trigger.as_str()));
    // Lock async: as streams do loop são owned, não dependem dele.
    let svc = svc.lock().await;
    let request = svc
        .shortcuts
        .bind_shortcuts(&svc.session, &[new_shortcut], None, BindShortcutsOptions::default())
        .await?;
    drop(svc);
    match request.response() {
        Ok(bound) => Ok(bound
            .shortcuts()
            .first()
            .map(|s| s.trigger_description().to_string())),
        Err(err) => Err(anyhow::anyhow!("portal: {err}")),
    }
}

/// Escuta o portal de atalhos globais (ashpd) e traduz
/// activated/deactivated/changed em mutações no [`Shared`].
pub async fn run(shared: Arc<std::sync::Mutex<Shared>>, preferred: String) -> anyhow::Result<()> {
    use futures_util::StreamExt;

    let svc = match ensure_service().await {
        Ok(svc) => svc,
        Err(err) => {
            set_shared(&shared, |s| {
                let lang = s.lang.clone();
                s.shortcut_error = Some(friendly_portal_error(&lang, &err));
                s.bump();
            });
            return Ok(());
        }
    };
    let lang = shared.lock().unwrap().lang.clone();
    match bind(&svc, &preferred, &lang).await {
        Ok(Some(trigger)) => {
            log::info!("atalho global vinculado: '{trigger}' (preferido '{preferred}')");
            // Portal pode retornar trigger vazio (ex. Hyprland) ou só após
            // diálogo do sistema — vazio não é atalho válido, mantém o
            // preferido na UI em vez de apagar.
            if trigger.trim().is_empty() {
                log::warn!("portal retornou trigger vazio; mantendo '{preferred}'");
            } else {
                set_shared(&shared, |s| {
                    s.shortcut = trigger;
                    s.shortcut_error = None;
                    s.bump();
                });
            }
        }
        Ok(None) => {
            log::warn!("portal não retornou atalho (bind pendente de diálogo?)");
        }
        Err(err) => {
            set_shared(&shared, |s| {
                let lang = s.lang.clone();
                s.shortcut_error = Some(friendly_portal_error(&lang, &err));
                s.bump();
            });
            return Ok(());
        }
    }

    let (mut activated, mut deactivated, mut changed) = {
        let svc = svc.lock().await;
        let activated = Box::pin(svc.shortcuts.receive_activated().await?.fuse());
        let deactivated = Box::pin(svc.shortcuts.receive_deactivated().await?.fuse());
        let changed = Box::pin(svc.shortcuts.receive_shortcuts_changed().await?.fuse());
        (activated, deactivated, changed)
    };

    loop {
        futures_util::select! {
            event = activated.next() => {
                let Some(event) = event else { break };
                log::info!("atalho activated: {}", event.shortcut_id());
                if event.shortcut_id() == OVERLAY_ID {
                    // Ativa na hora (fallback central no primário) e resolve
                    // a posição exata em thread dedicada (ver backend::overlay).
                    log::info!("atalho opts: {:?}", event.options());
                    crate::backend::overlay::open(&shared);
                }
            }
            event = deactivated.next() => {
                let Some(event) = event else { break };
                if event.shortcut_id() == OVERLAY_ID {
                    set_shared(&shared, |s| {
                        if s.overlay_active {
                            s.confirm_request = true;
                            s.bump();
                        }
                    });
                }
            }
            event = changed.next() => {
                let Some(event) = event else { break };
                if let Some(shortcut) = event.shortcuts().first() {
                    let trigger = shortcut.trigger_description().to_string();
                    set_shared(&shared, |s| {
                        s.shortcut = trigger.clone();
                        s.shortcut_error = None;
                        s.bump();
                    });
                    let mut settings = crate::core::settings::load();
                    settings.shortcut = trigger;
                    crate::core::settings::save(&settings);
                }
            }
        }
    }
    Ok(())
}

/// Reabre o fluxo de bind do portal (o popup de escolha do sistema).
/// Chamado pelo botão de atalho nas settings. Na mesma sessão do loop,
/// então o listener continua valendo.
///
/// NOTA: chamar `bind_shortcuts` de novo na mesma sessão não reabre o
/// diálogo do portal (retorna o binding atual) — por isso o botão parecia
/// não fazer nada. O caminho correto para "trocar" é `configure_shortcuts`,
/// que abre a UI de configuração; a escolha chega via sinal
/// `ShortcutsChanged` já escutado em `run()`.
pub async fn rebind(shared: Arc<std::sync::Mutex<Shared>>) -> anyhow::Result<()> {
    set_shared(&shared, |s| {
        s.shortcut_rebinding = true;
        s.bump();
    });
    let result = async {
        let svc = ensure_service().await?;
        // Tenta abrir a UI de configuração (portal v2+).
        let configured = {
            let svc = svc.lock().await;
            svc.shortcuts
                .configure_shortcuts(&svc.session, None, Default::default())
                .await
        };
        match configured {
            Ok(()) => Ok(None),
            // Portal antigo sem ConfigureShortcuts: cai para bind (mostra o
            // diálogo de primeira vinculação).
            Err(_) => {
                let (preferred, lang) = {
                    let s = shared.lock().unwrap();
                    (s.shortcut.clone(), s.lang.clone())
                };
                bind(&svc, &preferred, &lang).await
            }
        }
    }
    .await;
    let mut s = shared.lock().unwrap();
    match result {
        Ok(Some(trigger)) => {
            s.shortcut = trigger.clone();
            s.shortcut_error = None;
            let mut settings = crate::core::settings::load();
            settings.shortcut = trigger;
            crate::core::settings::save(&settings);
        }
        // ConfigureShortcuts é fire-and-forget: o novo trigger chega via
        // sinal ShortcutsChanged (tratado em `run()`).
        Ok(None) => {}
        Err(err) => {
            let msg = err.to_string();
            // Usuário fechou o diálogo sem escolher: não é erro.
            if !msg.contains("Cancelled") && !msg.contains("cancelled") {
                let lang = s.lang.clone();
                s.shortcut_error = Some(friendly_portal_error(&lang, &err));
            }
        }
    }
    s.shortcut_rebinding = false;
    s.bump();
    Ok(())
}

fn set_shared(shared: &Arc<std::sync::Mutex<Shared>>, f: impl FnOnce(&mut Shared)) {
    if let Ok(mut s) = shared.lock() {
        f(&mut s);
    }
}

/// Explica o erro mais comum sem sandbox (portal exige app-id).
fn friendly_portal_error(lang: &str, err: &anyhow::Error) -> String {
    let msg = err.to_string();
    if msg.contains("An app id is required") {
        t(lang, "err.shortcut_portal")
    } else {
        t_fmt(lang, "err.shortcut", &[("msg", &msg)])
    }
}

#[cfg(test)]
mod tests {
    use super::spec_trigger;

    #[test]
    fn trigger_sem_colchetes_e_mods_maiusculos() {
        assert_eq!(spec_trigger("Alt+Shift+S"), "ALT+SHIFT+S");
        assert_eq!(spec_trigger("[Alt+Shift+S]"), "ALT+SHIFT+S");
        assert_eq!(spec_trigger("ctrl+alt+a"), "CTRL+ALT+a");
        assert_eq!(spec_trigger("Super+Shift+X"), "LOGO+SHIFT+X");
    }

    /// Sonda: o portal GlobalShortcuts aceita chamador SEM sandbox?
    ///
    /// `run()` só retorna em erro fatal (ex. app-id); em sucesso escuta
    /// para sempre (diálogo do sistema pendente). Logo, retorno rápido +
    /// `last_error` = RECUSA; timeout = ACEITA (ainda escutando).
    /// Ignorado por padrão (precisa de sessão + pode abrir diálogo):
    /// `cargo test portal_bind -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn portal_bind_fora_do_sandbox() {
        use std::sync::{Arc, Mutex};
        use std::time::Duration;

        use futures_util::future::{select, Either};
        use futures_util::pin_mut;

        use crate::core::settings::Settings;
        use crate::core::state::Shared;

        let shared = Arc::new(Mutex::new(Shared::new(&Settings::default())));
        let outcome = async_io::block_on(async {
            let run = super::run(shared.clone(), "Alt+Shift+S".to_string());
            let wait = async_io::Timer::after(Duration::from_secs(8));
            pin_mut!(run);
            pin_mut!(wait);
            match select(run, wait).await {
                Either::Left(_) => "returned",
                Either::Right(_) => "listening",
            }
        });
        let err = shared.lock().unwrap().last_error.clone();
        println!("portal fora do sandbox: {outcome} last_error={err:?}");
        assert_eq!(outcome, "listening", "portal recusou: {err:?}");
    }
}
