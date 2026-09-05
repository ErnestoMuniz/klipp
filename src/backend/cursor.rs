/// Posição global do cursor via XWayland (XQueryPointer).
///
/// No Wayland puro o compositor não informa a posição do cursor a tomadores
/// (por design), então o overlay não sabe em qual monitor abrir. Mas numa
/// sessão KDE/GNOME com XWayland ativo, o ponteiro é espelhado no display X
/// (`:0`) e dá para ler com `XQueryPointer` — sem permissões extras, sem
/// portal, sem diálogo. Se não houver XWayland, retorna `None` e o overlay
/// usa o fallback (primário + primeiro mouse_move).
pub fn pointer() -> Option<(f32, f32)> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::query_pointer;

    // Sem DISPLAY não há nem XWayland para perguntar.
    if std::env::var_os("DISPLAY").is_none() {
        return None;
    }
    let (conn, screen) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen].root;
    let reply = query_pointer(&conn, root).ok()?.reply().ok()?;
    Some((reply.root_x as f32, reply.root_y as f32))
}

#[cfg(test)]
mod tests {
    use super::pointer;

    #[test]
    fn query_nao_trava_e_retorna_algo_plausivel() {
        if std::env::var_os("DISPLAY").is_none() {
            return;
        }
        let Some((x, y)) = pointer() else {
            return; // sem XWayland: fallback cobre
        };
        assert!(x >= 0.0 && y >= 0.0 && x < 20000.0 && y < 20000.0, "cursor: ({x}, {y})");
    }
}
