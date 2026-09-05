use std::sync::{Arc, OnceLock};

use ashpd::desktop::global_shortcuts::GlobalShortcuts;
use ashpd::desktop::Session;
use futures_util::lock::Mutex as AsyncMutex;

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
) -> anyhow::Result<Option<String>> {
    use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, NewShortcut};
    let trigger = spec_trigger(preferred);
    let new_shortcut =
        NewShortcut::new(OVERLAY_ID, "Abrir o seletor de sons").preferred_trigger(Some(trigger.as_str()));
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
                s.last_error = Some(friendly_portal_error(&err));
                s.bump();
            });
            return Ok(());
        }
    };
    match bind(&svc, &preferred).await {
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
                    s.bump();
                });
            }
        }
        Ok(None) => {
            log::warn!("portal não retornou atalho (bind pendente de diálogo?)");
        }
        Err(err) => {
            set_shared(&shared, |s| {
                s.last_error = Some(friendly_portal_error(&err));
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
                    // Posição do cursor via XWayland + offset calibrado: o pie
                    // já abre no monitor certo, perto do cursor. O primeiro
                    // mouse_move real confirma a posição exata (e aprende o
                    // offset). Sem XWayland, None + fallback (primário).
                    let x = crate::backend::cursor::pointer();
                    log::info!("atalho cursor: {x:?}");
                    set_shared(&shared, |s| {
                        let (ox, oy) = s.cursor_calib.unwrap_or((0.0, 0.0));
                        let anchor = x.map(|p| (p.0 + ox, p.1 + oy));
                        s.overlay_active = true;
                        s.overlay_anchor = anchor;
                        s.anchor_x = x;
                        s.anchor_needs_confirm = anchor.is_some();
                        s.pie_hovered = None;
                        s.confirm_request = false;
                        s.bump();
                    });
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
                let preferred = shared.lock().unwrap().shortcut.clone();
                bind(&svc, &preferred).await
            }
        }
    }
    .await;
    let mut s = shared.lock().unwrap();
    match result {
        Ok(Some(trigger)) => {
            s.shortcut = trigger.clone();
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
                s.last_error = Some(friendly_portal_error(&err));
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

/// Explica o erro mais comum fora do Flatpak (portal exige app-id).
fn friendly_portal_error(err: &anyhow::Error) -> String {
    let msg = err.to_string();
    if msg.contains("An app id is required") {
        "atalho global indisponível: o portal exige app-id — rode via Flatpak ou scripts/dev-run.sh".into()
    } else {
        format!("atalho: {msg}")
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
}
