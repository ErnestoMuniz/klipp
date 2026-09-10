/// Comando que um atalho custom do DE executa para alternar o overlay.
///
/// Usado no GNOME (gsettings) e no Hyprland (binding de config): o atalho
/// chama `klipp --toggle-overlay` e a instância única entrega o pedido via
/// IPC (ver `backend::ipc`).
pub fn toggle_command() -> String {
    if ashpd::is_sandboxed() {
        return "flatpak run io.github.ErnestoMuniz.Klipp --toggle-overlay".into();
    }
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        if !appimage.trim().is_empty() {
            return format!("{} --toggle-overlay", quote_path(&appimage));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let s = exe.to_string_lossy().into_owned();
        if !s.is_empty() {
            return format!("{} --toggle-overlay", quote_path(&s));
        }
    }
    "klipp --toggle-overlay".into()
}

pub fn quote_path(p: &str) -> String {
    if p.contains(' ') || p.contains('"') {
        format!("\"{}\"", p.replace('"', "\\\""))
    } else {
        p.to_string()
    }
}
