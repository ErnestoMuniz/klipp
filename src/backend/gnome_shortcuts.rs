use std::process::Command;
use std::sync::Arc;

use crate::core::i18n::{t, t_fmt};
use crate::core::state::Shared;

/// Atalho global no GNOME via atalho custom do sistema (gsettings).
///
/// O portal GlobalShortcuts exige app-id e só funciona empacotado
/// (sandbox/dev com `scripts/dev-run.sh`); o AppImage roda sem sandbox,
/// então no GNOME o app registra sozinho um atalho custom que executa
/// `klipp --toggle-overlay` — a instância única entrega ao processo
/// principal via socket (ver `backend::ipc`). Sem daemon, sem popup do
/// portal, configurável pelo app como no KDE.
///
/// Esquema (relocatable):
/// `org.gnome.settings-daemon.plugins.media-keys custom-keybindings`
/// lista os slots; cada slot
/// `org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:<slot>`
/// guarda `name`, `command` e `binding` (ex. `<Alt><Shift>s`).
pub const SHORTCUT_NAME: &str = "Klipp Overlay";
const SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const SLOT_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const SLOT_PREFIX: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/";

/// Fora de sandbox + GNOME: atalho custom via gsettings (no sandbox, portal).
pub fn using_gnome() -> bool {
    !ashpd::is_sandboxed() && is_desktop_gnome()
}

/// GNOME (inclui "ubuntu:GNOME", Pop!_OS): checa os marcadores de sessão.
pub fn is_desktop_gnome() -> bool {
    for key in [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
        "GDMSESSION",
    ] {
        if std::env::var(key)
            .map(|v| v.to_ascii_lowercase().contains("gnome"))
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// `"Alt+Shift+S"` → `"<Alt><Shift>s"`. Letras/dígitos exigem modificador
/// (sem isso o atalho sequestraria a digitação); F-teclas e especiais
/// valem sozinhas. `Err("invalid"|"modifier")` = não suportada.
pub fn spec_to_gnome(spec: &str) -> Result<String, &'static str> {
    let clean = spec.trim().trim_start_matches('[').trim_end_matches(']');
    let mut mods = String::new();
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut meta = false;
    let mut key: Option<&str> = None;
    for part in clean.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()) {
        match part.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" | "PRIMARY" => ctrl = true,
            "ALT" | "ALTGR" => alt = true,
            "SHIFT" => shift = true,
            "LOGO" | "SUPER" | "META" | "WIN" => meta = true,
            _ => {
                if key.is_some() {
                    return Err("invalid");
                }
                key = Some(part);
            }
        }
    }
    let gnome_key = gnome_key(key.ok_or("invalid")?)?;
    if !ctrl && !alt && !shift && !meta && needs_modifier(&gnome_key) {
        return Err("modifier");
    }
    // Ordem canônica do GTK: Control, Alt, Shift, Super.
    if ctrl {
        mods.push_str("<Control>");
    }
    if alt {
        mods.push_str("<Alt>");
    }
    if shift {
        mods.push_str("<Shift>");
    }
    if meta {
        mods.push_str("<Super>");
    }
    Ok(format!("{mods}{gnome_key}"))
}

/// `"<Alt><Shift>s"` → `"Alt+Shift+S"` (ordem Ctrl, Alt, Shift, Meta,
/// igual ao `qt_decode`). `None` = formato desconhecido.
pub fn gnome_to_spec(binding: &str) -> Option<String> {
    let b = binding.trim().trim_matches('\'').trim();
    if b.is_empty() {
        return None;
    }
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut meta = false;
    let mut rest = b;
    // Modificadores `<...>` na frente; o que sobra é a tecla.
    while let Some(inner) = rest.strip_prefix('<') {
        let end = inner.find('>')?;
        let (name, after) = inner.split_at(end);
        match name.to_ascii_lowercase().as_str() {
            "control" | "ctrl" | "primary" => ctrl = true,
            "alt" | "mod1" => alt = true,
            "shift" => shift = true,
            "super" | "meta" | "win" | "mod4" => meta = true,
            _ => return None,
        }
        rest = &after[1..];
    }
    if rest.is_empty() || rest.contains('<') || rest.contains('>') {
        return None;
    }
    let key = display_key(rest)?;
    let mut parts: Vec<String> = Vec::new();
    if ctrl {
        parts.push("Ctrl".to_string());
    }
    if alt {
        parts.push("Alt".to_string());
    }
    if shift {
        parts.push("Shift".to_string());
    }
    if meta {
        parts.push("Meta".to_string());
    }
    parts.push(key);
    Some(parts.join("+"))
}

fn gnome_key(name: &str) -> Result<String, &'static str> {
    let upper = name.to_ascii_uppercase();
    if upper.chars().count() == 1 {
        let c = upper.chars().next().unwrap();
        if c.is_ascii_alphanumeric() {
            return Ok(c.to_ascii_lowercase().to_string());
        }
        return Ok(match c {
            ' ' => "space".to_string(),
            ',' => "comma".to_string(),
            '.' => "period".to_string(),
            '/' => "slash".to_string(),
            '\\' => "backslash".to_string(),
            ';' => "semicolon".to_string(),
            '\'' => "apostrophe".to_string(),
            '[' => "bracketleft".to_string(),
            ']' => "bracketright".to_string(),
            '-' => "minus".to_string(),
            '=' => "equal".to_string(),
            '`' => "grave".to_string(),
            _ => return Err("invalid"),
        });
    }
    match upper.as_str() {
        "SPACE" => Ok("space".to_string()),
        "ESC" | "ESCAPE" => Ok("Escape".to_string()),
        "TAB" => Ok("Tab".to_string()),
        "BACKSPACE" => Ok("BackSpace".to_string()),
        "ENTER" | "RETURN" => Ok("Return".to_string()),
        "INSERT" => Ok("Insert".to_string()),
        "DELETE" => Ok("Delete".to_string()),
        "HOME" => Ok("Home".to_string()),
        "END" => Ok("End".to_string()),
        "PAGEUP" => Ok("Page_Up".to_string()),
        "PAGEDOWN" => Ok("Page_Down".to_string()),
        "UP" => Ok("Up".to_string()),
        "DOWN" => Ok("Down".to_string()),
        "LEFT" => Ok("Left".to_string()),
        "RIGHT" => Ok("Right".to_string()),
        _ => {
            if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
                if (1..=12).contains(&n) {
                    return Ok(format!("F{n}"));
                }
            }
            Err("invalid")
        }
    }
}

fn display_key(gnome: &str) -> Option<String> {
    if gnome.chars().count() == 1 {
        let c = gnome.chars().next().unwrap();
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase().to_string());
        }
        return None;
    }
    Some(
        match gnome.to_ascii_lowercase().as_str() {
            "space" => "Space",
            "escape" => "Esc",
            "tab" => "Tab",
            "backspace" => "BackSpace",
            "return" | "enter" | "kp_enter" => "Enter",
            "insert" => "Insert",
            "delete" => "Delete",
            "home" => "Home",
            "end" => "End",
            "page_up" => "PageUp",
            "page_down" => "PageDown",
            "up" => "Up",
            "down" => "Down",
            "left" => "Left",
            "right" => "Right",
            "comma" => ",",
            "period" => ".",
            "slash" => "/",
            "backslash" => "\\",
            "semicolon" => ";",
            "apostrophe" => "'",
            "bracketleft" => "[",
            "bracketright" => "]",
            "minus" => "-",
            "equal" => "=",
            "grave" => "`",
            _ => {
                let upper = gnome.to_ascii_uppercase();
                if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
                    if (1..=12).contains(&n) {
                        return Some(format!("F{n}"));
                    }
                }
                return None;
            }
        }
        .to_string(),
    )
}

fn needs_modifier(gnome_key: &str) -> bool {
    // Imprimível sozinha sequestraria a digitação; especiais valem puras.
    gnome_key.chars().count() == 1
        || matches!(
            gnome_key,
            "space"
                | "comma"
                | "period"
                | "slash"
                | "backslash"
                | "semicolon"
                | "apostrophe"
                | "bracketleft"
                | "bracketright"
                | "minus"
                | "equal"
                | "grave"
        )
}

/// Comando que o atalho custom executa. No AppImage prefere `$APPIMAGE`
/// (o mount em /tmp muda a cada boot); no sandbox, `flatpak run`.
/// Compartilhado com o caminho do Hyprland (`backend::hyprland_shortcuts`).
pub use crate::backend::custom_shortcut::toggle_command;

fn gsettings_get(schema_path: &str, key: &str) -> anyhow::Result<String> {
    let out = Command::new("gsettings")
        .arg("get")
        .arg(schema_path)
        .arg(key)
        .output()
        .map_err(|e| anyhow::anyhow!("gsettings indisponível: {e}"))?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "gsettings get {key}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn gsettings_set(schema_path: &str, key: &str, value: &str) -> anyhow::Result<()> {
    let out = Command::new("gsettings")
        .arg("set")
        .arg(schema_path)
        .arg(key)
        .arg(value)
        .output()
        .map_err(|e| anyhow::anyhow!("gsettings indisponível: {e}"))?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "gsettings set {key}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// Desempacota string GVariant (`'foo\'bar'` → `foo'bar`).
fn unquote(raw: &str) -> String {
    let t = raw.trim();
    if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
        t[1..t.len() - 1].replace("\\'", "'").replace("\\\\", "\\")
    } else {
        t.to_string()
    }
}

/// `"['/a/custom0/', '/a/custom1/']"` (ou `@as []`) → slots.
fn parse_slot_list(raw: &str) -> Vec<String> {
    let mut slots = Vec::new();
    let mut cur: Option<String> = None;
    for c in raw.chars() {
        match (cur.as_mut(), c) {
            (None, '\'') => cur = Some(String::new()),
            (Some(s), '\'') => {
                slots.push(std::mem::take(s));
                cur = None;
            }
            (Some(s), _) => s.push(c),
            (None, _) => {}
        }
    }
    slots
}

fn format_slot_list(slots: &[String]) -> String {
    if slots.is_empty() {
        return "[]".to_string();
    }
    let inner: Vec<String> = slots.iter().map(|s| format!("'{s}'")).collect();
    format!("[{}]", inner.join(", "))
}

fn slot_schema(slot: &str) -> String {
    format!("{SLOT_SCHEMA}:{slot}")
}

fn list_slots() -> anyhow::Result<Vec<String>> {
    Ok(parse_slot_list(&gsettings_get(
        SCHEMA,
        "custom-keybindings",
    )?))
}

/// Slot do Klipp: comando com `klipp` + `toggle-overlay`, ou nome conhecido.
fn find_klipp_slot(slots: &[String]) -> Option<String> {
    let mut name_match = None;
    for slot in slots {
        let cmd = gsettings_get(&slot_schema(slot), "command")
            .map(|r| unquote(&r))
            .unwrap_or_default();
        let low = cmd.to_ascii_lowercase();
        if low.contains("klipp") && low.contains("toggle-overlay") {
            return Some(slot.clone());
        }
        if name_match.is_none() {
            let name = gsettings_get(&slot_schema(slot), "name")
                .map(|r| unquote(&r))
                .unwrap_or_default();
            if name.to_ascii_lowercase().contains("klipp") {
                name_match = Some(slot.clone());
            }
        }
    }
    name_match
}

fn alloc_slot(slots: &[String]) -> String {
    let mut i = 0u32;
    loop {
        let slot = format!("{SLOT_PREFIX}custom{i}/");
        if !slots.contains(&slot) {
            return slot;
        }
        i += 1;
    }
}

/// Conflito com outro binding do sistema: o GNOME ignora silencioso o
/// novo (atalho "morto"), então barra na hora em vez de fingir que salvou.
fn binding_taken_elsewhere(new_binding: &str, our_slot: &str) -> bool {
    let needle = format!("'{new_binding}'");
    let Ok(out) = Command::new("gsettings").arg("list-recursively").output() else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let ours = format!("{SLOT_SCHEMA}:{our_slot} binding ");
    for line in stdout.lines() {
        if line.contains(&needle) && !line.starts_with(&ours) {
            return true;
        }
    }
    false
}

/// Slot do Klipp + binding GNOME bruto. `None` = sem vínculo.
fn klipp_slot() -> Option<(String, String)> {
    let slots = list_slots().ok()?;
    let slot = find_klipp_slot(&slots)?;
    let binding = unquote(&gsettings_get(&slot_schema(&slot), "binding").ok()?);
    if binding.is_empty() {
        return None;
    }
    Some((slot, binding))
}

fn slot_command(slot: &str) -> String {
    gsettings_get(&slot_schema(slot), "command")
        .map(|r| unquote(&r))
        .unwrap_or_default()
}

fn is_klipp_command(cmd: &str) -> bool {
    let low = cmd.to_ascii_lowercase();
    low.contains("klipp") && low.contains("toggle-overlay")
}

/// Sincroniza na largada: adota o vínculo do sistema, ou instala o
/// preferido (primeira execução). Sem loop: o atalho chama
/// `klipp --toggle-overlay`, que chega via IPC (ver `backend::ipc`).
pub async fn run(shared: Arc<std::sync::Mutex<Shared>>) -> anyhow::Result<()> {
    match klipp_slot() {
        Some((slot, binding)) => {
            let display = gnome_to_spec(&binding).unwrap_or_else(|| binding.clone());
            log::info!("atalho gnome atual: '{display}'");
            set_shared(&shared, |s| {
                if s.shortcut != display {
                    s.shortcut = display.clone();
                    s.bump();
                }
            });
            // O binário mudou (ex. AppImage nova versão, debug → release):
            // atualiza o comando do nosso vínculo, sem tocar no binding.
            let cmd = slot_command(&slot);
            if is_klipp_command(&cmd) && cmd != toggle_command() {
                match gsettings_set(&slot_schema(&slot), "command", &toggle_command()) {
                    Ok(()) => log::info!("atalho gnome: comando atualizado"),
                    Err(err) => log::warn!("atalho gnome: comando desatualizado: {err}"),
                }
            }
            persist_shortcut(&shared);
        }
        None => {
            let (preferred, lang) = {
                let s = shared.lock().unwrap();
                (s.shortcut.clone(), s.lang.clone())
            };
            match install(&preferred) {
                Ok(display) => {
                    log::info!("atalho gnome instalado: '{display}'");
                    set_shared(&shared, |s| {
                        s.shortcut = display.clone();
                        s.shortcut_error = None;
                        s.bump();
                    });
                    persist_shortcut(&shared);
                }
                Err(err) => {
                    log::warn!("atalho gnome indisponível: {err}");
                    set_shared(&shared, |s| {
                        s.shortcut_error = Some(gnome_error(&lang, &err));
                        s.bump();
                    });
                }
            }
        }
    }
    Ok(())
}

/// Troca o atalho em tempo de execução (gravador in-app): valida, grava
/// no gsettings, persiste. `Err` = em uso ("taken"), inválido ("invalid")
/// ou falha do gsettings (mensagem pronta).
pub async fn set_shortcut(
    shared: &Arc<std::sync::Mutex<Shared>>,
    spec: &str,
) -> anyhow::Result<String> {
    let display = install(spec).map_err(|e| {
        let msg = e.to_string();
        if msg == "taken" || msg == "invalid" || msg == "modifier" {
            anyhow::anyhow!(msg)
        } else {
            e
        }
    })?;
    set_shared(shared, |s| {
        s.shortcut = display.clone();
        s.shortcut_error = None;
        s.bump();
    });
    persist_shortcut(shared);
    Ok(display)
}

fn install(spec: &str) -> anyhow::Result<String> {
    let binding = spec_to_gnome(spec).map_err(|kind| anyhow::anyhow!(kind))?;
    let mut slots = list_slots()?;
    let slot = find_klipp_slot(&slots).unwrap_or_else(|| alloc_slot(&slots));
    if !slots.contains(&slot) {
        slots.push(slot.clone());
        gsettings_set(SCHEMA, "custom-keybindings", &format_slot_list(&slots))?;
    }
    let old = gsettings_get(&slot_schema(&slot), "binding")
        .map(|r| unquote(&r))
        .unwrap_or_default();
    // Conflito com outro atalho custom: compara slot a slot (o
    // `list-recursively` global não inclui schemas relocatable).
    for other in &slots {
        if *other == slot {
            continue;
        }
        let other_binding = gsettings_get(&slot_schema(other), "binding")
            .map(|r| unquote(&r))
            .unwrap_or_default();
        if !other_binding.is_empty() && other_binding.eq_ignore_ascii_case(&binding) {
            return Err(anyhow::anyhow!("taken"));
        }
    }
    // Conflito com binding nativo do sistema: o GNOME ignora silencioso
    // o novo (atalho "morto"), então barra na hora em vez de fingir.
    if old != binding && binding_taken_elsewhere(&binding, &slot) {
        return Err(anyhow::anyhow!("taken"));
    }
    let schema = slot_schema(&slot);
    gsettings_set(&schema, "name", SHORTCUT_NAME)?;
    gsettings_set(&schema, "command", &toggle_command())?;
    gsettings_set(&schema, "binding", &binding)?;
    // GNOME normaliza na leitura; vazio = rejeitou o nome da tecla.
    let back = unquote(&gsettings_get(&schema, "binding")?);
    if back.is_empty() {
        return Err(anyhow::anyhow!("invalid"));
    }
    Ok(gnome_to_spec(&back).unwrap_or_else(|| {
        // Binding aceito mas fora do nosso mapa (ex. tecla multimídia
        // editada à mão): exibe o pedido normalizado.
        gnome_to_spec(&binding).unwrap_or_else(|| spec.to_string())
    }))
}

/// Erro com o caminho manual (Settings → Teclado → Atalhos custom).
pub fn gnome_error(lang: &str, err: &anyhow::Error) -> String {
    let msg = err.to_string();
    if msg == "taken" {
        t(lang, "err.shortcut_taken")
    } else if msg == "invalid" || msg == "modifier" {
        t(lang, "err.shortcut_invalid")
    } else {
        t_fmt(
            lang,
            "err.shortcut_gnome",
            &[("msg", &msg), ("cmd", &toggle_command())],
        )
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

#[cfg(test)]
mod tests {
    use super::{
        binding_taken_elsewhere, display_key, format_slot_list, gnome_key, gnome_to_spec,
        parse_slot_list, spec_to_gnome,
    };

    #[test]
    fn codec_padrao_e_variacoes() {
        assert_eq!(spec_to_gnome("Alt+Shift+S").unwrap(), "<Alt><Shift>s");
        assert_eq!(spec_to_gnome("[Alt+Shift+S]").unwrap(), "<Alt><Shift>s");
        assert_eq!(spec_to_gnome("ctrl+alt+a").unwrap(), "<Control><Alt>a");
        assert_eq!(spec_to_gnome("Super+Shift+X").unwrap(), "<Shift><Super>x");
        assert_eq!(spec_to_gnome("Ctrl+Alt+,").unwrap(), "<Control><Alt>comma");
        assert_eq!(spec_to_gnome("F12").unwrap(), "F12");
        assert_eq!(gnome_to_spec("<Alt><Shift>s").unwrap(), "Alt+Shift+S");
        assert_eq!(gnome_to_spec("<Control><Alt>a").unwrap(), "Ctrl+Alt+A");
        assert_eq!(gnome_to_spec("<Primary><Alt>t").unwrap(), "Ctrl+Alt+T");
        assert_eq!(gnome_to_spec("F12").unwrap(), "F12");
        assert_eq!(gnome_to_spec("<Super>F1").unwrap(), "Meta+F1");
    }

    #[test]
    fn codec_ida_e_volta() {
        for spec in [
            "Alt+Shift+S",
            "Ctrl+Alt+T",
            "Ctrl+Shift+F5",
            "Alt+F4",
            "Ctrl+Alt+Delete",
            "Alt+Space",
        ] {
            let gnome = spec_to_gnome(spec).unwrap();
            assert_eq!(gnome_to_spec(&gnome).unwrap(), spec, "volta de {gnome}");
        }
    }

    #[test]
    fn codec_recusa_tecla_pura_e_lixo() {
        assert!(spec_to_gnome("S").is_err());
        assert!(spec_to_gnome("space").is_err());
        assert!(spec_to_gnome("").is_err());
        assert!(spec_to_gnome("Alt+").is_err());
        assert!(spec_to_gnome("Alt+Foo").is_err());
        assert!(spec_to_gnome("Alt+S+D").is_err());
        assert!(gnome_to_spec("").is_none());
        assert!(gnome_to_spec("<Hyper>s").is_none());
        assert!(gnome_to_spec("s<t").is_none());
    }

    #[test]
    fn chaves_nomeadas_nos_dois_sentidos() {
        assert_eq!(gnome_key("PageUp").unwrap(), "Page_Up");
        assert_eq!(gnome_key("Escape").unwrap(), "Escape");
        assert_eq!(display_key("Page_Up").unwrap(), "PageUp");
        assert_eq!(display_key("BackSpace").unwrap(), "BackSpace");
    }

    #[test]
    fn lista_de_slots() {
        assert!(parse_slot_list("@as []").is_empty());
        assert!(parse_slot_list("[]").is_empty());
        assert_eq!(
            parse_slot_list("['/a/custom0/', '/a/custom1/']"),
            vec!["/a/custom0/".to_string(), "/a/custom1/".to_string()]
        );
        assert_eq!(format_slot_list(&[]), "[]");
        assert_eq!(
            format_slot_list(&["/a/custom0/".to_string()]),
            "['/a/custom0/']"
        );
    }

    #[test]
    fn conflito_ignora_o_proprio_slot() {
        // Sem gsettings real aqui: só garante que a função não explode
        // quando o binário falta (assume livre em vez de bloquear).
        let _ = binding_taken_elsewhere("<Control><Alt>z", "/nonexistent/");
    }

    /// Ao vivo: `install` + `current_binding` de verdade, com limpeza.
    /// Cobre vínculo novo, idempotência (reinstalar o próprio não é
    /// "em uso") e conflito com outro slot custom. Restaura a lista
    /// exata de antes. Precisa de GNOME com gsettings:
    /// `cargo test gnome_instala -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn gnome_instala_le_e_restaura() {
        use std::sync::{Arc, Mutex};

        use crate::core::settings::Settings;
        use crate::core::state::Shared;

        let before = super::list_slots().expect("ler slots");
        // Slot Klipp pré-existente (ex. do próprio app): sai da lista
        // durante o teste e volta idêntico no cleanup.
        let pre_slot = super::find_klipp_slot(&before);
        let pre_backup = pre_slot.as_ref().map(|slot| {
            let schema = super::slot_schema(slot);
            let get = |key: &str| {
                super::gsettings_get(&schema, key)
                    .map(|r| super::unquote(&r))
                    .unwrap_or_default()
            };
            (slot.clone(), get("name"), get("command"), get("binding"))
        });
        if let Some(slot) = &pre_slot {
            let without: Vec<String> = before.iter().filter(|s| *s != slot).cloned().collect();
            assert!(without.len() + 1 == before.len());
            super::gsettings_set(
                super::SCHEMA,
                "custom-keybindings",
                &super::format_slot_list(&without),
            )
            .expect("afastar slot pré-existente");
        }
        // `run`/`install` persistem em settings.json: salva para restaurar.
        let settings_path = crate::core::settings::settings_path();
        let settings_before = std::fs::read(&settings_path).ok();
        // Cleanup incondicional (vale para Ok, Err e panic): relista,
        // reseta chaves estranhas, restaura backup + settings.
        let cleanup = || {
            let _ = super::gsettings_set(
                super::SCHEMA,
                "custom-keybindings",
                &super::format_slot_list(&before),
            );
            for slot in super::list_slots().unwrap_or_default() {
                if before.contains(&slot) {
                    continue;
                }
                let schema = super::slot_schema(&slot);
                for key in ["name", "command", "binding"] {
                    let _ = std::process::Command::new("gsettings")
                        .arg("reset")
                        .arg(&schema)
                        .arg(key)
                        .status();
                }
            }
            // O teste pode ter reusado o path do slot pré-existente
            // (delistado no início): restaura as chaves do backup.
            if let Some((slot, name, command, binding)) = &pre_backup {
                let schema = super::slot_schema(slot);
                let _ = super::gsettings_set(&schema, "name", name);
                let _ = super::gsettings_set(&schema, "command", command);
                let _ = super::gsettings_set(&schema, "binding", binding);
            }
            match &settings_before {
                Some(raw) => {
                    let _ = std::fs::write(&settings_path, raw);
                }
                None => {
                    let _ = std::fs::remove_file(&settings_path);
                }
            }
        };

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // 1. Vínculo novo (slot pré-existente afastado no início; o
            // path pode ser reusado — o que vale é o conteúdo).
            let klipp_display = || super::klipp_slot().and_then(|(_, b)| super::gnome_to_spec(&b));
            assert!(klipp_display().is_none(), "sem vínculo no início");
            let display = super::install("Ctrl+Alt+U")?;
            assert_eq!(display, "Ctrl+Alt+U");
            assert_eq!(klipp_display().as_deref(), Some("Ctrl+Alt+U"));
            // 2. Reinstalar o próprio não é conflito.
            assert_eq!(super::install("Ctrl+Alt+U")?, "Ctrl+Alt+U");
            // 3. Outro slot com o mesmo binding barra como "em uso".
            let rival = super::alloc_slot(&super::list_slots()?);
            let mut with = super::list_slots()?;
            with.push(rival.clone());
            super::gsettings_set(
                super::SCHEMA,
                "custom-keybindings",
                &super::format_slot_list(&with),
            )?;
            let rschema = super::slot_schema(&rival);
            super::gsettings_set(&rschema, "name", "Rival")?;
            super::gsettings_set(&rschema, "command", "true")?;
            super::gsettings_set(&rschema, "binding", "<Control><Alt>u")?;
            let err = super::install("Ctrl+Alt+U").unwrap_err().to_string();
            assert_eq!(err, "taken", "conflito devia barrar");
            // Rival removido da lista (as chaves órfãs são resetadas no
            // cleanup): o vínculo volta a ser instalável.
            let mut with = super::list_slots()?;
            with.retain(|s| s != &rival);
            super::gsettings_set(
                super::SCHEMA,
                "custom-keybindings",
                &super::format_slot_list(&with),
            )?;
            assert_eq!(super::install("Ctrl+Alt+U")?, "Ctrl+Alt+U");
            // 4. Comando obsoleto no nosso slot: `run` atualiza o comando
            // sem trocar o binding e adota a exibição no Shared.
            let slot = super::find_klipp_slot(&super::list_slots()?).expect("slot klipp");
            let schema = super::slot_schema(&slot);
            super::gsettings_set(&schema, "command", "klipp --toggle-overlay")?;
            let shared = Arc::new(Mutex::new(Shared::new(&Settings::default())));
            async_io::block_on(super::run(shared.clone())).expect("run adota o vínculo");
            assert_eq!(super::slot_command(&slot), super::toggle_command());
            assert_eq!(shared.lock().unwrap().shortcut, "Ctrl+Alt+U");
            assert!(shared.lock().unwrap().shortcut_error.is_none());
            Ok::<(), anyhow::Error>(())
        }));

        cleanup();
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(err)) => panic!("falha no teste ao vivo: {err}"),
            Err(_) => panic!("panic no teste ao vivo (estado restaurado)"),
        }
        assert_eq!(super::list_slots().expect("reler slots"), before);
        if let Some((slot, name, command, binding)) = &pre_backup {
            let schema = super::slot_schema(slot);
            let get = |key: &str| {
                super::gsettings_get(&schema, key)
                    .map(|r| super::unquote(&r))
                    .unwrap_or_default()
            };
            assert_eq!(&get("name"), name);
            assert_eq!(&get("command"), command);
            assert_eq!(&get("binding"), binding);
        }
    }
}
