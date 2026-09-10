use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use crate::core::i18n::{t, t_fmt};
use crate::core::state::Shared;

/// Atalho global no Hyprland via binding de config do compositor.
///
/// O portal GlobalShortcuts não serve aqui: o `xdg-desktop-portal-hyprland`
/// ignora o preferred trigger (registra a ação com trigger vazio), então a
/// tecla tem que estar no config. O app instala/atualiza um bloco gerenciado
/// no config do usuário (Lua no Hyprland/Omarchy novo, `.conf` no parser
/// legado) e chama `hyprctl reload`: o reload limpa todos os binds `__lua`,
/// recria o estado Lua e re-executa o config, então a tecla nova passa a
/// valer na hora — sem logout.
pub const BIND_DESCRIPTION: &str = "Klipp overlay";

const MARK_BEGIN_LUA: &str = "-- >>> klipp overlay (managed by klipp; do not edit)";
const MARK_END_LUA: &str = "-- <<< klipp overlay";
const MARK_BEGIN_CONF: &str = "# >>> klipp overlay (managed by klipp; do not edit)";
const MARK_END_CONF: &str = "# <<< klipp overlay";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Style {
    Lua,
    Legacy,
}

/// Fora de sandbox + Hyprland: binding de config (no sandbox, portal).
pub fn using_hyprland() -> bool {
    !ashpd::is_sandboxed() && is_hyprland()
}

pub fn is_hyprland() -> bool {
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    for key in ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"] {
        if std::env::var(key)
            .map(|v| v.to_ascii_lowercase().contains("hyprland"))
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// `hyprctl binds -j` → spec do binding do Klipp, se houver.
pub fn current_binding() -> Option<String> {
    let out = Command::new("hyprctl").args(["binds", "-j"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let binds: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    for bind in binds.as_array()? {
        let desc = bind.get("description").and_then(|v| v.as_str()).unwrap_or("");
        // No parser Lua o `arg` é um índice interno; no legado é o comando.
        let arg = bind.get("arg").and_then(|v| v.as_str()).unwrap_or("");
        let low = format!("{desc} {arg}").to_ascii_lowercase();
        if !low.contains("klipp") {
            continue;
        }
        let key = bind.get("key").and_then(|v| v.as_str()).unwrap_or("");
        let modmask = bind.get("modmask").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        if let Some(spec) = bind_to_spec(modmask, key) {
            return Some(spec);
        }
    }
    None
}

/// Modmask do Hyprland: SHIFT=1, CAPS=2, CTRL=4, ALT=8, MOD2=16, MOD3=32,
/// SUPER=64. Ordem de exibição igual ao resto do app (Ctrl, Alt, Shift, Meta).
fn bind_to_spec(modmask: u32, key: &str) -> Option<String> {
    if key.is_empty() || key.starts_with("mouse:") || key.starts_with("code:") {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    if modmask & 4 != 0 {
        parts.push("Ctrl".into());
    }
    if modmask & 8 != 0 {
        parts.push("Alt".into());
    }
    if modmask & 1 != 0 {
        parts.push("Shift".into());
    }
    if modmask & 64 != 0 {
        parts.push("Meta".into());
    }
    parts.push(display_key(key)?);
    Some(parts.join("+"))
}

fn display_key(hypr: &str) -> Option<String> {
    let named = match hypr.to_ascii_lowercase().as_str() {
        "space" => "Space",
        "escape" | "esc" => "Esc",
        "tab" => "Tab",
        "backspace" => "BackSpace",
        "return" | "enter" => "Enter",
        "insert" => "Insert",
        "delete" => "Delete",
        "home" => "Home",
        "end" => "End",
        "page_up" | "prior" => "PageUp",
        "page_down" | "next" => "PageDown",
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
        _ => "",
    };
    if !named.is_empty() {
        return Some(named.to_string());
    }
    let upper = hypr.to_ascii_uppercase();
    if upper.chars().count() == 1 && upper.chars().next().unwrap().is_ascii_alphanumeric() {
        return Some(upper);
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        if (1..=12).contains(&n) {
            return Some(format!("F{n}"));
        }
    }
    None
}

/// `"Alt+Shift+S"` → `"ALT + SHIFT + S"` (formato do `hl.bind`).
/// `None` = combinação não suportada.
pub fn spec_to_hyprland(spec: &str) -> Option<String> {
    let (mods, key) = split_spec(spec)?;
    let key = hypr_key(key)?;
    // Ordem canônica, igual aos outros backends.
    let mut parts: Vec<String> = Vec::new();
    if mods.0 {
        parts.push("CTRL".into());
    }
    if mods.1 {
        parts.push("ALT".into());
    }
    if mods.2 {
        parts.push("SHIFT".into());
    }
    if mods.3 {
        parts.push("SUPER".into());
    }
    parts.push(key);
    Some(parts.join(" + "))
}

/// `"Alt+Shift+S"` → `"ALT SHIFT, S"` (formato do `bind =` legado).
fn spec_to_legacy(spec: &str) -> Option<String> {
    let (mods, key) = split_spec(spec)?;
    let key = hypr_key(key)?;
    let mut m: Vec<&str> = Vec::new();
    if mods.0 {
        m.push("CTRL");
    }
    if mods.1 {
        m.push("ALT");
    }
    if mods.2 {
        m.push("SHIFT");
    }
    if mods.3 {
        m.push("SUPER");
    }
    if m.is_empty() {
        return Some(key);
    }
    Some(format!("{}, {}", m.join(" "), key))
}

type Mods = (bool, bool, bool, bool); // ctrl, alt, shift, super

fn split_spec(spec: &str) -> Option<(Mods, &str)> {
    let clean = spec.trim().trim_start_matches('[').trim_end_matches(']');
    let mut mods: Mods = (false, false, false, false);
    let mut key: Option<&str> = None;
    for part in clean.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()) {
        match part.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods.0 = true,
            "ALT" | "ALTGR" => mods.1 = true,
            "SHIFT" => mods.2 = true,
            "LOGO" | "SUPER" | "META" | "WIN" => mods.3 = true,
            _ => {
                if key.is_some() {
                    return None;
                }
                key = Some(part);
            }
        }
    }
    Some((mods, key?))
}

fn hypr_key(name: &str) -> Option<String> {
    let upper = name.to_ascii_uppercase();
    if upper.chars().count() == 1 {
        let c = upper.chars().next().unwrap();
        if c.is_ascii_alphanumeric() {
            return Some(c.to_string());
        }
        return Some(
            match c {
                ' ' => "SPACE",
                ',' => "comma",
                '.' => "period",
                '/' => "slash",
                '\\' => "backslash",
                ';' => "semicolon",
                '\'' => "apostrophe",
                '[' => "bracketleft",
                ']' => "bracketright",
                '-' => "minus",
                '=' => "equal",
                '`' => "grave",
                _ => return None,
            }
            .to_string(),
        );
    }
    Some(
        match upper.as_str() {
            "SPACE" => "SPACE",
            "ESC" | "ESCAPE" => "ESCAPE",
            "TAB" => "TAB",
            "BACKSPACE" => "BACKSPACE",
            "ENTER" | "RETURN" => "RETURN",
            "INSERT" => "INSERT",
            "DELETE" => "DELETE",
            "HOME" => "HOME",
            "END" => "END",
            "PAGEUP" => "PAGE_UP",
            "PAGEDOWN" => "PAGE_DOWN",
            "UP" => "UP",
            "DOWN" => "DOWN",
            "LEFT" => "LEFT",
            "RIGHT" => "RIGHT",
            other => {
                if let Some(n) = other.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
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

/// Nomes de keysym aceitos por `hl.is_key_down` para a tecla do atalho
/// (formato normalizado do `hypr_key`). Devolve os dois casos quando diferem
/// (sem/com Shift), o Lua testa os dois — cobre Shift e CapsLock.
fn keysym_candidates(bind_key: &str) -> Vec<String> {
    let upper = bind_key.to_ascii_uppercase();
    if upper.chars().count() == 1 {
        let c = upper.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            // XKB_KEY_a e XKB_KEY_A são keysyms distintos.
            return vec![c.to_ascii_lowercase().to_string(), c.to_string()];
        }
        if c.is_ascii_digit() {
            let shifted = match c {
                '1' => "exclam",
                '2' => "at",
                '3' => "numbersign",
                '4' => "dollar",
                '5' => "percent",
                '6' => "asciicircum",
                '7' => "ampersand",
                '8' => "asterisk",
                '9' => "parenleft",
                '0' => "parenright",
                _ => unreachable!(),
            };
            return vec![c.to_string(), shifted.to_string()];
        }
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        if (1..=12).contains(&n) {
            return vec![format!("F{n}")];
        }
    }
    let pair = match upper.as_str() {
        "SPACE" => ("space", "space"),
        "ESC" | "ESCAPE" => ("Escape", "Escape"),
        "TAB" => ("Tab", "ISO_Left_Tab"),
        "BACKSPACE" => ("BackSpace", "BackSpace"),
        "ENTER" | "RETURN" => ("Return", "Return"),
        "INSERT" => ("Insert", "Insert"),
        "DELETE" => ("Delete", "Delete"),
        "HOME" => ("Home", "Home"),
        "END" => ("End", "End"),
        "PAGEUP" | "PAGE_UP" => ("Page_Up", "Page_Up"),
        "PAGEDOWN" | "PAGE_DOWN" => ("Page_Down", "Page_Down"),
        "UP" => ("Up", "Up"),
        "DOWN" => ("Down", "Down"),
        "LEFT" => ("Left", "Left"),
        "RIGHT" => ("Right", "Right"),
        "COMMA" => ("comma", "less"),
        "PERIOD" => ("period", "greater"),
        "SLASH" => ("slash", "question"),
        "BACKSLASH" => ("backslash", "bar"),
        "SEMICOLON" => ("semicolon", "colon"),
        "APOSTROPHE" => ("apostrophe", "quotedbl"),
        "BRACKETLEFT" => ("bracketleft", "braceleft"),
        "BRACKETRIGHT" => ("bracketright", "braceright"),
        "MINUS" => ("minus", "underscore"),
        "EQUAL" => ("equal", "plus"),
        "GRAVE" => ("grave", "asciitilde"),
        _ => return Vec::new(),
    };
    vec![pair.0.to_string(), pair.1.to_string()]
}

/// Escapa uma string para virar literal Lua entre aspas duplas.
fn lua_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

/// Bloco gerenciado. No Lua: o press abre o overlay e arma um timer que faz
/// polling do release com `hl.is_key_down`, confirmando quando a tecla é
/// solta. No parser legado: `bind` + `bindr` (melhor esforço).
///
/// O `bindr` do Hyprland não serve para atalho com modificadores: no release
/// ele exige que o modmask atual bata com o do press, então soltar Alt/Shift
/// antes da tecla principal perde o evento (ver `CKeybindManager`). O polling
/// lê o estado físico da tecla e é imune à ordem dos modificadores.
fn bind_block_for(style: Style, shortcut: &str) -> Option<String> {
    let pressed = lua_escape(&crate::backend::custom_shortcut::pressed_command());
    let released = lua_escape(&crate::backend::custom_shortcut::released_command());
    match style {
        Style::Lua => {
            let key = hypr_key(split_spec(shortcut)?.1)?;
            let trigger = spec_to_hyprland(shortcut)?;
            let candidates = keysym_candidates(&key);
            let down = if candidates.is_empty() {
                "true".to_string()
            } else {
                candidates
                    .iter()
                    .map(|k| format!("hl.is_key_down(\"{k}\")"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            };
            Some(format!(
                "local klipp_hold = nil\n\
                 hl.bind(\"{trigger}\", function()\n\
                 \u{20} hl.exec_cmd(\"{pressed}\")\n\
                 \u{20} if klipp_hold then klipp_hold:set_enabled(false) end\n\
                 \u{20} local t\n\
                 \u{20} t = hl.timer(function()\n\
                 \u{20}   if not ({down}) then\n\
                 \u{20}     t:set_enabled(false)\n\
                 \u{20}     if klipp_hold == t then klipp_hold = nil end\n\
                 \u{20}     hl.exec_cmd(\"{released}\")\n\
                 \u{20}   end\n\
                 \u{20} end, {{ timeout = 50, type = \"repeat\" }})\n\
                 \u{20} klipp_hold = t\n\
                 end, {{ description = \"{BIND_DESCRIPTION}\" }})"
            ))
        }
        Style::Legacy => {
            let trigger = spec_to_legacy(shortcut)?;
            let pressed = crate::backend::custom_shortcut::pressed_command();
            let released = crate::backend::custom_shortcut::released_command();
            Some(format!(
                "bind = {trigger}, exec, {pressed}\n\
                 bindr = {trigger}, exec, {released}"
            ))
        }
    }
}

fn markers(style: Style) -> (&'static str, &'static str) {
    match style {
        Style::Lua => (MARK_BEGIN_LUA, MARK_END_LUA),
        Style::Legacy => (MARK_BEGIN_CONF, MARK_END_CONF),
    }
}

/// Arquivo de config do Hyprland onde instalar. No Lua prefere o
/// `bindings.lua` (Omarchy), depois o `hyprland.lua`; por fim o `.conf`
/// legado. `None` = não achou config para editar (cai na instrução manual).
fn target_file() -> Option<(PathBuf, Style)> {
    let dir = dirs::config_dir()?.join("hypr");
    let bindings = dir.join("bindings.lua");
    if bindings.is_file() {
        return Some((bindings, Style::Lua));
    }
    let lua_main = dir.join("hyprland.lua");
    if lua_main.is_file() {
        return Some((lua_main, Style::Lua));
    }
    let conf = dir.join("hyprland.conf");
    if conf.is_file() {
        return Some((conf, Style::Legacy));
    }
    None
}

/// Instala/atualiza o bloco gerenciado. Devolve se o arquivo mudou.
fn install(shortcut: &str) -> anyhow::Result<bool> {
    let (path, style) = target_file()
        .ok_or_else(|| anyhow::anyhow!("config do Hyprland não encontrado"))?;
    let body = bind_block_for(style, shortcut).ok_or_else(|| anyhow::anyhow!("invalid"))?;
    let (begin, end) = markers(style);
    let block = format!("{begin}\n{body}\n{end}");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = upsert_block(&content, &block, begin, end);
    if updated != content {
        std::fs::write(&path, updated)?;
        log::info!("atalho hyprland: config atualizado em {}", path.display());
        return Ok(true);
    }
    Ok(false)
}

/// Recarrega o config. O reload do Hyprland limpa os binds `__lua`, recria o
/// estado Lua e re-executa o config — por isso a tecla nova passa a valer
/// sem logout. A chamada é síncrona (o compositor termina antes de responder).
fn reload() {
    match Command::new("hyprctl").arg("reload").output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => log::warn!(
            "atalho hyprland: hyprctl reload: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(err) => log::warn!("atalho hyprland: hyprctl indisponível: {err}"),
    }
}

/// Troca o bloco entre `begin`/`end` ou acrescenta no fim.
fn upsert_block(content: &str, block: &str, begin: &str, end: &str) -> String {
    if let Some(start) = content.find(begin) {
        if let Some(end_rel) = content[start..].find(end) {
            let end_abs = start + end_rel + end.len();
            return format!("{}{}{}", &content[..start], block, &content[end_abs..]);
        }
    }
    let mut out = content.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(block);
    out.push('\n');
    out
}

/// Sincroniza na largada: adota o binding ativo, ou instala/atualiza o
/// bloco gerenciado no config e recarrega. O `hyprctl reload` aplica na hora.
pub async fn run(shared: Arc<std::sync::Mutex<Shared>>) -> anyhow::Result<()> {
    let active = current_binding();
    let shortcut = match &active {
        Some(spec) => spec.clone(),
        None => shared.lock().unwrap().shortcut.clone(),
    };
    let wrote = match install(&shortcut) {
        Ok(wrote) => wrote,
        Err(err) => {
            // Sem config para editar: se já existe um vínculo ativo, só
            // adota; senão cai na instrução manual.
            log::warn!("atalho hyprland: instalação automática falhou: {err}");
            false
        }
    };
    // Recarrega se o config mudou ou se ainda não há vínculo ativo (bloco
    // já presente mas a sessão ainda não o carregou).
    let applied = if wrote || active.is_none() {
        reload();
        current_binding().or(active)
    } else {
        active
    };
    match applied {
        Some(spec) => {
            log::info!("atalho hyprland atual: '{spec}'");
            set_shared(&shared, |s| {
                if s.shortcut != spec {
                    s.shortcut = spec.clone();
                    s.bump();
                }
                s.shortcut_error = None;
            });
            persist_shortcut(&shared);
        }
        None if wrote => {
            set_shared(&shared, |s| {
                s.shortcut = shortcut.clone();
                s.bump();
            });
            persist_shortcut(&shared);
            log::info!(
                "atalho hyprland: config escrito, mas o reload não confirmou; reinicie a sessão"
            );
            set_restart_note(&shared);
        }
        None => set_manual_error(&shared, &shortcut),
    }
    Ok(())
}

/// Troca em tempo de execução (gravador in-app): escreve o config e
/// persiste. `Err("invalid")` = combinação não suportada; a aplicação
/// efetiva exige reiniciar a sessão (aviso no `shortcut_error`).
pub async fn set_shortcut(
    shared: &Arc<std::sync::Mutex<Shared>>,
    spec: &str,
) -> anyhow::Result<String> {
    if spec_to_hyprland(spec).is_none() {
        return Err(anyhow::anyhow!("invalid"));
    }
    install(spec)?;
    reload();
    // O reload recria os binds `__lua`; se não confirmar, cai no aviso de
    // reiniciar (fallback raro).
    let applied = current_binding().is_some();
    set_shared(shared, |s| {
        s.shortcut = spec.to_string();
        s.shortcut_error = if applied {
            None
        } else {
            Some(restart_note(&s.lang))
        };
        s.bump();
    });
    persist_shortcut(shared);
    Ok(spec.to_string())
}

/// Aviso: config escrito, falta reiniciar a sessão do Hyprland.
pub fn restart_note(lang: &str) -> String {
    t(lang, "err.shortcut_hyprland_restart")
}

fn set_restart_note(shared: &Arc<std::sync::Mutex<Shared>>) {
    if let Ok(mut s) = shared.lock() {
        s.shortcut_error = Some(restart_note(&s.lang));
        s.bump();
    }
}

/// Instrução manual (sem config editável ou combo inválida).
pub fn manual_error(lang: &str, shortcut: &str) -> String {
    t_fmt(
        lang,
        "err.shortcut_hyprland",
        &[("line", &bind_line(shortcut))],
    )
}

fn set_manual_error(shared: &Arc<std::sync::Mutex<Shared>>, shortcut: &str) {
    if let Ok(mut s) = shared.lock() {
        s.shortcut_error = Some(manual_error(&s.lang, shortcut));
        s.bump();
    }
}

/// Bloco sugerido (formato Lua) para a instrução manual e para logs.
pub fn bind_line(shortcut: &str) -> String {
    bind_block_for(Style::Lua, shortcut).unwrap_or_else(|| shortcut.to_string())
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
    use super::{Style, bind_block_for, bind_to_spec, spec_to_hyprland, spec_to_legacy, upsert_block};

    #[test]
    fn spec_vira_trigger_do_hyprland() {
        assert_eq!(spec_to_hyprland("Alt+Shift+S").unwrap(), "ALT + SHIFT + S");
        assert_eq!(spec_to_hyprland("[Alt+Shift+S]").unwrap(), "ALT + SHIFT + S");
        assert_eq!(spec_to_hyprland("ctrl+alt+a").unwrap(), "CTRL + ALT + A");
        assert_eq!(spec_to_hyprland("Super+Shift+X").unwrap(), "SHIFT + SUPER + X");
        assert_eq!(spec_to_hyprland("F12").unwrap(), "F12");
        assert_eq!(spec_to_legacy("Alt+Shift+S").unwrap(), "ALT SHIFT, S");
        assert!(spec_to_hyprland("Alt+").is_none());
        assert!(spec_to_hyprland("Alt+S+D").is_none());
    }

    #[test]
    fn bind_do_hyprctl_vira_spec() {
        // SHIFT=1, CTRL=4, ALT=8, SUPER=64.
        assert_eq!(bind_to_spec(9, "S").as_deref(), Some("Alt+Shift+S"));
        assert_eq!(bind_to_spec(12, "u").as_deref(), Some("Ctrl+Alt+U"));
        assert_eq!(bind_to_spec(64, "PERIOD").as_deref(), Some("Meta+."));
        assert_eq!(bind_to_spec(0, "F12").as_deref(), Some("F12"));
        assert!(bind_to_spec(0, "mouse:272").is_none());
        assert!(bind_to_spec(0, "code:10").is_none());
    }

    #[test]
    fn bloco_lua_faz_polling_do_release() {
        // Press abre; um timer lê o estado físico da tecla e confirma na
        // soltura (o bindr do Hyprland perde o release se um modificador
        // for solto antes da tecla principal).
        let lua = bind_block_for(Style::Lua, "Alt+Shift+S").unwrap();
        assert!(lua.contains("--overlay-pressed"), "{lua}");
        assert!(lua.contains("--overlay-released"), "{lua}");
        assert!(
            lua.contains("hl.is_key_down(\"s\") or hl.is_key_down(\"S\")"),
            "{lua}"
        );
        assert!(lua.contains("type = \"repeat\""), "{lua}");
        assert!(!lua.contains("release = true"), "sem bindr: {lua}");

        let conf = bind_block_for(Style::Legacy, "Alt+Shift+S").unwrap();
        assert!(conf.starts_with("bind = ALT SHIFT, S, exec, "), "{conf}");
        assert!(conf.contains("--overlay-pressed"));
        assert!(conf.contains("\nbindr = ALT SHIFT, S, exec, "), "{conf}");
        assert!(conf.contains("--overlay-released"));
    }

    #[test]
    fn keysym_cobre_letras_digitos_e_nomes() {
        use super::keysym_candidates;
        assert_eq!(keysym_candidates("S"), vec!["s", "S"]);
        assert_eq!(keysym_candidates("1"), vec!["1", "exclam"]);
        assert_eq!(keysym_candidates("SPACE"), vec!["space", "space"]);
        assert_eq!(keysym_candidates("PAGE_UP"), vec!["Page_Up", "Page_Up"]);
        assert_eq!(keysym_candidates("comma"), vec!["comma", "less"]);
        assert_eq!(keysym_candidates("F12"), vec!["F12"]);
    }

    #[test]
    fn bloco_gerenciado_e_substituido_sem_duplicar() {
        let begin = "-- >>> klipp overlay";
        let end = "-- <<< klipp overlay";
        let block = format!("{begin}\nlinha nova\n{end}");
        let base = "outra_coisa()\n";
        let once = upsert_block(base, &block, begin, end);
        assert!(once.contains("linha nova"));
        assert_eq!(once.matches(begin).count(), 1);
        // Substituir com uma tecla nova não duplica nem apaga o resto.
        let block2 = format!("{begin}\nlinha mais nova\n{end}");
        let twice = upsert_block(&once, &block2, begin, end);
        assert!(twice.contains("linha mais nova"));
        assert!(!twice.contains("linha nova"));
        assert!(twice.starts_with("outra_coisa()"));
        assert_eq!(twice.matches(begin).count(), 1);
    }
}
