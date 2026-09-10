/// Comando que um atalho custom do DE executa para alternar o overlay.
///
/// Usado no GNOME (gsettings): o atalho chama `klipp --toggle-overlay` e a
/// instância única entrega o pedido via IPC (ver `backend::ipc`).
pub fn toggle_command() -> String {
    overlay_command("--toggle-overlay")
}

/// Comando do atalho de "segurar" no Hyprland: abre o overlay (equivale ao
/// `globalShortcutPressed` do KDE). Quem solta chama [`released_command`].
pub fn pressed_command() -> String {
    overlay_command("--overlay-pressed")
}

/// Comando do atalho de "soltar" no Hyprland: confirma a seleção (equivale
/// ao `globalShortcutReleased` do KDE).
pub fn released_command() -> String {
    overlay_command("--overlay-released")
}

/// `klipp <flag>` resolvendo o binário certo (Flatpak, AppImage ou exe).
fn overlay_command(flag: &str) -> String {
    if ashpd::is_sandboxed() {
        return format!("flatpak run io.github.ErnestoMuniz.Klipp {flag}");
    }
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        if !appimage.trim().is_empty() {
            return format!("{} {flag}", quote_path(&appimage));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let s = exe.to_string_lossy().into_owned();
        if !s.is_empty() {
            return format!("{} {flag}", quote_path(&s));
        }
    }
    format!("klipp {flag}")
}

pub fn quote_path(p: &str) -> String {
    if p.contains(' ') || p.contains('"') {
        format!("\"{}\"", p.replace('"', "\\\""))
    } else {
        p.to_string()
    }
}
