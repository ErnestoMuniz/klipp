use std::sync::{Arc, OnceLock};

use futures_util::lock::Mutex as AsyncMutex;
use futures_util::StreamExt;

use crate::core::i18n::t;
use crate::core::state::Shared;

/// Atalho global nativo do KDE via KGlobalAccel D-Bus
/// (`org.kde.kglobalaccel`, provido pelo KWin no Plasma 6).
///
/// Usado fora de sandbox (AppImage/binário): o portal GlobalShortcuts
/// exige app-id e só funciona empacotado. Aqui o app registra a ação
/// direto no daemon — configurar pelo app, sem atalho manual no DE.
///
/// Protocolo (mesmo de outros clientes não-Qt):
/// `doRegister([comp, comp_friendly, action, action_friendly])`,
/// `setShortcut(id, [qt_key], PRESENT)` e escuta de
/// `globalShortcutPressed/Released` em `org.kde.kglobalaccel.Component`.
pub const COMPONENT_UNIQUE: &str = "io.github.ErnestoMuniz.Klipp";
const COMPONENT_FRIENDLY: &str = "Klipp";
pub const ACTION_UNIQUE: &str = "klipp-overlay";
/// Nome fixo em inglês: faz parte da identidade (mudar orfana o registro).
const ACTION_FRIENDLY: &str = "Open sound picker";
/// Flag do `setShortcut`: atalho presente (persiste em kglobalshortcutsrc).
const FLAG_PRESENT: u32 = 0x02;

// Qt key encoding: modificadores ORados com o código da tecla
// (QKeySequence de 1 combinação). Letras/dígitos = ASCII maiúsculo.
const MOD_SHIFT: i32 = 0x0200_0000;
const MOD_CTRL: i32 = 0x0400_0000;
const MOD_ALT: i32 = 0x0800_0000;
const MOD_META: i32 = 0x1000_0000;

fn action_id() -> [&'static str; 4] {
    [
        COMPONENT_UNIQUE,
        COMPONENT_FRIENDLY,
        ACTION_UNIQUE,
        ACTION_FRIENDLY,
    ]
}

/// Fora de sandbox + KDE: atalho nativo vale (no sandbox, portal).
pub fn using_native() -> bool {
    !ashpd::is_sandboxed() && is_kde()
}

fn is_kde() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("XDG_SESSION_DESKTOP"))
        .map(|v| v.to_ascii_lowercase().contains("kde"))
        .unwrap_or(false)
}

#[zbus::proxy(
    interface = "org.kde.KGlobalAccel",
    default_service = "org.kde.kglobalaccel",
    default_path = "/kglobalaccel"
)]
trait KGlobalAccel {
    #[zbus(name = "doRegister")]
    fn do_register(&self, id: &[&str]) -> zbus::Result<()>;
    #[zbus(name = "setShortcut")]
    fn set_shortcut(
        &self,
        action_id: &[&str],
        keys: &[i32],
        flags: u32,
    ) -> zbus::Result<Vec<i32>>;
    #[zbus(name = "unRegister")]
    fn un_register(&self, action_id: &[&str]) -> zbus::Result<()>;
    #[zbus(name = "shortcut")]
    fn get_shortcut(&self, action_id: &[&str]) -> zbus::Result<Vec<i32>>;
}

type ConnCell = Arc<AsyncMutex<Option<zbus::Connection>>>;
static SERVICE: OnceLock<ConnCell> = OnceLock::new();

fn cell() -> ConnCell {
    SERVICE
        .get_or_init(|| Arc::new(AsyncMutex::new(None)))
        .clone()
}

async fn ensure_connection() -> anyhow::Result<zbus::Connection> {
    let cell = cell();
    let mut guard = cell.lock().await;
    if let Some(conn) = guard.clone() {
        return Ok(conn);
    }
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| anyhow::anyhow!("dbus: {e}"))?;
    *guard = Some(conn.clone());
    Ok(conn)
}

/// `"Alt+Shift+S"` → `0x0A000053`. Letras/dígitos/espaço exigem modificador
/// (sem isso o atalho sequestraria a digitação); F-teclas valem sozinhas.
pub fn qt_encode(spec: &str) -> Result<i32, &'static str> {
    let clean = spec.trim().trim_start_matches('[').trim_end_matches(']');
    let mut mods = 0i32;
    let mut key: Option<&str> = None;
    for part in clean.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()) {
        match part.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods |= MOD_CTRL,
            "ALT" | "ALTGR" => mods |= MOD_ALT,
            "SHIFT" => mods |= MOD_SHIFT,
            "LOGO" | "SUPER" | "META" | "WIN" => mods |= MOD_META,
            _ => {
                if key.is_some() {
                    return Err("invalid");
                }
                key = Some(part);
            }
        }
    }
    let code = qt_key(key.ok_or("empty")?)?;
    if mods == 0 && needs_modifier(code) {
        return Err("modifier");
    }
    Ok(mods | code)
}

fn qt_key(name: &str) -> Result<i32, &'static str> {
    let upper = name.to_ascii_uppercase();
    if upper.chars().count() == 1 {
        let c = upper.chars().next().unwrap();
        // Qt usa o próprio ASCII para teclas imprimíveis (letras, dígitos,
        // pontuação): vale para o daemon sem tabela extra.
        if c.is_ascii_graphic() {
            return Ok(c as i32);
        }
        if c == ' ' {
            return Ok(0x20);
        }
        return Err("invalid");
    }
    match upper.as_str() {
        "SPACE" => Ok(0x20),
        "ESC" | "ESCAPE" => Ok(0x0100_0000),
        "TAB" => Ok(0x0100_0001),
        "BACKSPACE" => Ok(0x0100_0003),
        "ENTER" | "RETURN" => Ok(0x0100_0004),
        "INSERT" => Ok(0x0100_0006),
        "DELETE" => Ok(0x0100_0007),
        "HOME" => Ok(0x0100_0010),
        "END" => Ok(0x0100_0011),
        "PAGEUP" => Ok(0x0100_0016),
        "PAGEDOWN" => Ok(0x0100_0017),
        "UP" => Ok(0x0100_0013),
        "DOWN" => Ok(0x0100_0015),
        "LEFT" => Ok(0x0100_0012),
        "RIGHT" => Ok(0x0100_0014),
        _ => {
            if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
                if (1..=12).contains(&n) {
                    return Ok(0x0100_0030 + (n as i32 - 1));
                }
            }
            Err("invalid")
        }
    }
}

/// `0x0A000053` → `"Alt+Shift+S"` (ordem canônica Ctrl, Alt, Shift, Meta).
pub fn qt_decode(code: i32) -> String {
    let mut parts: Vec<String> = Vec::new();
    if code & MOD_CTRL != 0 {
        parts.push("Ctrl".to_string());
    }
    if code & MOD_ALT != 0 {
        parts.push("Alt".to_string());
    }
    if code & MOD_SHIFT != 0 {
        parts.push("Shift".to_string());
    }
    if code & MOD_META != 0 {
        parts.push("Meta".to_string());
    }
    let key = code & !(MOD_CTRL | MOD_ALT | MOD_SHIFT | MOD_META);
    parts.push(match key {
        0x20 => "Space".to_string(),
        0x0100_0000 => "Esc".to_string(),
        0x0100_0001 => "Tab".to_string(),
        0x0100_0003 => "Backspace".to_string(),
        0x0100_0004 => "Enter".to_string(),
        0x0100_0006 => "Insert".to_string(),
        0x0100_0007 => "Delete".to_string(),
        0x0100_0010 => "Home".to_string(),
        0x0100_0011 => "End".to_string(),
        0x0100_0016 => "PageUp".to_string(),
        0x0100_0017 => "PageDown".to_string(),
        0x0100_0012 => "Left".to_string(),
        0x0100_0014 => "Right".to_string(),
        0x0100_0013 => "Up".to_string(),
        0x0100_0015 => "Down".to_string(),
        k if (0x0100_0030..=0x0100_003B).contains(&k) => {
            format!("F{}", k - 0x0100_0030 + 1)
        }
        k if (0x21..=0x7E).contains(&k) => ((k as u8) as char).to_string(),
        k => format!("0x{k:X}"),
    });
    parts.join("+")
}

fn needs_modifier(code: i32) -> bool {
    // Tecla imprimível sozinha sequestraria a digitação (vale para letras,
    // dígitos, espaço e pontuação); especiais (F-teclas, Enter…) valem puras.
    let key = code & !(MOD_CTRL | MOD_ALT | MOD_SHIFT | MOD_META);
    key < 0x0100_0000
}

/// Registra a ação, concilia com o daemon (mudança externa em
/// System Settings vence) e traduz pressed/released em mutações no [`Shared`].
pub async fn run(shared: Arc<std::sync::Mutex<Shared>>) -> anyhow::Result<()> {
    let conn = ensure_connection().await?;
    let proxy = KGlobalAccelProxy::new(&conn).await.map_err(dbus_err)?;
    let id = action_id();
    proxy.do_register(&id).await.map_err(dbus_err)?;

    let preferred = shared.lock().unwrap().shortcut.clone();
    match proxy.get_shortcut(&id).await.map_err(dbus_err) {
        Ok(current) if !current.is_empty() => {
            // Daemon já tem vínculo (primeira execução ou edição externa):
            // adota como verdade e persiste.
            let display = qt_decode(current[0]);
            log::info!("atalho nativo atual: '{display}'");
            set_shared(&shared, |s| {
                if s.shortcut != display {
                    s.shortcut = display.clone();
                    s.bump();
                }
            });
            persist_shortcut(&shared);
        }
        _ => {
            apply_preferred(&proxy, &shared, &preferred).await;
        }
    }

    listen(&conn, &shared).await;
    Ok(())
}

/// Troca o atalho em tempo de execução (gravador in-app): valida, aplica
/// no daemon, persiste. `Err` = em uso (retorno vazio) ou inválido.
///
/// NOTA KWin: `setShortcut` sobre vínculo vivo é ignorado (devolve o
/// atual) — trocar exige `unRegister` + `doRegister` + `set` de novo.
pub async fn set_shortcut(
    shared: &Arc<std::sync::Mutex<Shared>>,
    spec: &str,
) -> anyhow::Result<String> {
    let code = qt_encode(spec).map_err(|_| anyhow::anyhow!("invalid"))?;
    let conn = ensure_connection().await?;
    let proxy = KGlobalAccelProxy::new(&conn).await.map_err(dbus_err)?;
    let id = action_id();
    proxy.do_register(&id).await.map_err(dbus_err)?;
    if let Some(display) = try_set(&proxy, shared, &id, code).await {
        return Ok(display);
    }
    // Devolve o vínculo atual em vez de trocar: re-registra e repete.
    let previous = proxy.get_shortcut(&id).await.unwrap_or_default();
    let _ = proxy.un_register(&id).await;
    proxy.do_register(&id).await.map_err(dbus_err)?;
    if let Some(display) = try_set(&proxy, shared, &id, code).await {
        return Ok(display);
    }
    // Sem atalho (conflito de verdade): tenta restaurar o anterior para
    // não largar o usuário sem vínculo.
    if !previous.is_empty() {
        let _ = proxy.set_shortcut(&id, &previous, FLAG_PRESENT).await;
    }
    Err(anyhow::anyhow!("taken"))
}

/// Uma tentativa de `setShortcut`: `Some(display)` se o daemon assumiu.
async fn try_set(
    proxy: &KGlobalAccelProxy<'_>,
    shared: &Arc<std::sync::Mutex<Shared>>,
    id: &[&str; 4],
    code: i32,
) -> Option<String> {
    match proxy.set_shortcut(id, &[code], FLAG_PRESENT).await {
        Ok(actual) if actual.first().copied() == Some(code) => {
            let display = qt_decode(code);
            log::info!("atalho nativo: '{display}'");
            set_shared(shared, |s| {
                s.shortcut = display.clone();
                s.bump();
            });
            persist_shortcut(shared);
            Some(display)
        }
        Ok(actual) => {
            log::warn!("daemon devolveu {actual:?} para {code:#X}");
            None
        }
        Err(err) => {
            log::warn!("setShortcut: {err}");
            None
        }
    }
}

async fn apply_preferred(proxy: &KGlobalAccelProxy<'_>, shared: &Arc<std::sync::Mutex<Shared>>, preferred: &str) {
    let lang = shared.lock().unwrap().lang.clone();
    let code = match qt_encode(preferred) {
        Ok(code) => code,
        Err(_) => {
            set_shared(shared, |s| {
                s.shortcut_error = Some(t(&lang, "err.shortcut_invalid"));
                s.bump();
            });
            return;
        }
    };
    let id = action_id();
    match proxy.set_shortcut(&id, &[code], FLAG_PRESENT).await {
        Ok(actual) if actual.first().copied() == Some(code) => {
            log::info!("atalho nativo vinculado: '{preferred}'");
        }
        _ => {
            set_shared(shared, |s| {
                s.shortcut_error = Some(t(&lang, "err.shortcut_taken"));
                s.bump();
            });
        }
    }
}

/// Escuta pressed (abre) e released (confirma, como soltar o atalho)
/// numa única stream: o filtro por membro + ação acontece no loop.
async fn listen(conn: &zbus::Connection, shared: &Arc<std::sync::Mutex<Shared>>) {
    use zbus::message::Type::Signal;
    let rule = match zbus::MatchRule::builder()
        .msg_type(Signal)
        .interface("org.kde.kglobalaccel.Component")
    {
        Ok(b) => b.build(),
        Err(err) => {
            log::warn!("atalho nativo: regra inválida: {err}");
            return;
        }
    };
    let mut stream = match zbus::MessageStream::for_match_rule(rule, conn, None).await {
        Ok(stream) => stream,
        Err(err) => {
            log::warn!("atalho nativo indisponível: {err}");
            return;
        }
    };
    while let Some(msg) = stream.next().await {
        let Ok(msg) = msg else { continue };
        let member = msg
            .header()
            .member()
            .as_ref()
            .map(|m| m.to_string())
            .unwrap_or_default();
        let Ok((first, second, _ts)) = msg.body().deserialize::<(String, String, i64)>() else {
            continue;
        };
        // O par (componente, ação) varia com a versão: o kglobalacceld
        // clássico manda (component_unique, action_unique); o KWin6
        // embutido manda (component_unique, component_friendly).
        // Aceita se QUALQUER um bate (componente de ação única).
        if first != COMPONENT_UNIQUE && second != ACTION_UNIQUE {
            continue;
        }
        match member.as_str() {
            "globalShortcutPressed" => {
                log::info!("atalho nativo pressed");
                crate::backend::overlay::open(shared);
            }
            // Released sem pressed anterior é no-op: o filtro confere
            // `overlay_active` antes de confirmar.
            "globalShortcutReleased" => {
                log::info!("atalho nativo released");
                set_shared(shared, |s| {
                    if s.overlay_active {
                        s.confirm_request = true;
                        s.bump();
                    }
                });
            }
            _ => {}
        }
    }
}

fn set_shared(shared: &Arc<std::sync::Mutex<Shared>>, f: impl FnOnce(&mut Shared)) {
    if let Ok(mut s) = shared.lock() {
        f(&mut s);
    }
}

fn persist_shortcut(shared: &Arc<std::sync::Mutex<Shared>>) {
    let s = shared.lock().unwrap();
    crate::core::settings::save(&crate::core::settings::Settings {
        shortcut: s.shortcut.clone(),
        density: s.density.clone(),
        sort: s.sort.clone(),
        theme: s.theme.clone(),
        language: s.lang.clone(),
        mic_passthrough: s.mic_pass,
        hear_clips: s.hear_clips,
        run_in_background: s.run_in_background,
        show_hints: s.show_hint,
        volume: s.volume,
        mic_source: s.mic_source.clone(),
    });
}

fn dbus_err(e: zbus::Error) -> anyhow::Error {
    anyhow::anyhow!("dbus: {e}")
}

#[cfg(test)]
mod tests {
    use super::{qt_decode, qt_encode};

    #[test]
    fn qtcodec_padrao_e_variacoes() {
        assert_eq!(qt_encode("Alt+Shift+S").unwrap(), 0x0A00_0053);
        assert_eq!(qt_encode("[Alt+Shift+S]").unwrap(), 0x0A00_0053);
        assert_eq!(qt_encode("ctrl+alt+a").unwrap(), 0x0C00_0041);
        assert_eq!(qt_encode("Super+Shift+X").unwrap(), 0x1200_0058);
        assert_eq!(qt_encode("F12").unwrap(), 0x0100_003B);
        assert_eq!(qt_decode(0x0A00_0053), "Alt+Shift+S");
        assert_eq!(qt_decode(0x0400_0041), "Ctrl+A");
        assert_eq!(qt_decode(0x0100_003B), "F12");
    }

    #[test]
    fn qtcodec_recusa_tecla_pura_e_lixo() {
        assert!(qt_encode("S").is_err());
        assert!(qt_encode("space").is_err());
        assert!(qt_encode(",").is_err());
        assert!(qt_encode("").is_err());
        assert!(qt_encode("Alt+").is_err());
        assert!(qt_encode("Alt+Foo").is_err());
        assert!(qt_encode("Alt+S+D").is_err());
    }

    #[test]
    fn qtcodec_ctrl_alt_letra_e_pontuacao() {
        // `key_char` com Ctrl vira controle: o gravador usa o `key`
        // ("t") — o codec aceita igual.
        assert_eq!(qt_encode("Ctrl+Alt+T").unwrap(), 0x0C00_0054);
        assert_eq!(qt_decode(0x0C00_0054), "Ctrl+Alt+T");
        assert_eq!(qt_encode("Ctrl+Alt+,").unwrap(), 0x0C00_002C);
        assert_eq!(qt_decode(0x0C00_002C), "Ctrl+Alt+,");
    }

    /// Regressão ao vivo: trocar o vínculo DUAS vezes seguidas no daemon.
    /// (O KWin ignora `setShortcut` sobre vínculo vivo — sem o caminho
    /// `unRegister`, toda troca parecia "já em uso".)
    /// Precisa de KDE ativo; isola prefs via env e restaura o vínculo:
    /// `XDG_CONFIG_HOME=/tmp/fakecfg KLIPP_T=1 cargo test troca -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn troca_de_atalho_duas_vezes_no_daemon() {
        use std::sync::{Arc, Mutex};

        use crate::core::settings::Settings;
        use crate::core::state::Shared;

        let shared = Arc::new(Mutex::new(Shared::new(&Settings::default())));
        async_io::block_on(async {
            let a = super::set_shortcut(&shared, "Ctrl+Alt+U")
                .await
                .expect("primeira troca");
            assert_eq!(a, "Ctrl+Alt+U");
            let b = super::set_shortcut(&shared, "Ctrl+Alt+Y")
                .await
                .expect("segunda troca");
            assert_eq!(b, "Ctrl+Alt+Y");
            // Restaura o vínculo anterior do usuário.
            let _ = super::set_shortcut(&shared, "Ctrl+Alt+A").await;
        });
    }
}
