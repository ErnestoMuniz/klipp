use std::time::Duration;

/// Posição global do cursor via XWayland (XQueryPointer).
///
/// No Wayland puro o compositor não informa a posição do cursor a tomadores
/// (por design), então o overlay não sabe em qual monitor abrir. Mas numa
/// sessão KDE/GNOME com XWayland ativo, o ponteiro é espelhado no display X
/// e dá para ler com `XQueryPointer` — sem permissões extras, sem portal,
/// sem diálogo.
///
/// Pegadinha: `$DISPLAY` pode apontar para um Xorg obsoleto (ex. resto do
/// SDDM no `:0`) com o cursor congelado. Por isso `discover()` escolhe o
/// display X cuja raiz tem o tamanho do desktop Wayland (o XWayland espelha
/// todos os outputs). Sem candidato, retorna `None` e o overlay usa o
/// fallback (primário + primeiro mouse_move).
pub fn discover(desktop: (u32, u32)) -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(d) = std::env::var("DISPLAY") {
        if !d.trim().is_empty() {
            candidates.push(d);
        }
    }
    if let Ok(entries) = std::fs::read_dir("/tmp/.X11-unix") {
        for entry in entries.flatten() {
            if let Some(num) = entry
                .file_name()
                .to_str()
                .and_then(|n| n.strip_prefix('X'))
            {
                if num.parse::<u32>().is_ok() {
                    let d = format!(":{num}");
                    if !candidates.contains(&d) {
                        candidates.push(d);
                    }
                }
            }
        }
    }
    log::info!("cursor: displays X candidatos: {candidates:?} (desktop {desktop:?})");
    let mut best: Option<(i32, String)> = None;
    for display in candidates {
        match probe_one(&display, desktop) {
            Some((score, pos)) => {
                log::info!("cursor: X {display} raiz candidata, score {score}, cursor {pos:?}");
                if best.as_ref().is_none_or(|(s, _)| score > *s) {
                    best = Some((score, display));
                }
            }
            None => log::info!("cursor: X {display} inacessível"),
        }
    }
    let found = best.map(|(_, d)| d);
    log::info!("cursor: display X escolhido: {found:?}");
    found
}

#[cfg(test)]
mod tests {
    use super::{discover, pointer_on};

    #[test]
    fn query_invalido_retorna_none_sem_travar() {
        assert!(pointer_on(Some(":199")).is_none());
    }

    #[test]
    fn sonda_nao_trava_sem_x() {
        // Só checa que a sonda termina (valor depende do ambiente).
        let _ = discover((1920, 1080));
    }
}

/// Lê o cursor num display X explícito (`None` = `$DISPLAY` padrão).
pub fn pointer_on(display: Option<&str>) -> Option<(f32, f32)> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::query_pointer;

    let (conn, screen) = x11rb::connect(display).ok()?;
    let root = conn.setup().roots[screen].root;
    let reply = query_pointer(&conn, root).ok()?.reply().ok()?;
    Some((reply.root_x as f32, reply.root_y as f32))
}

fn probe_one(display: &str, desktop: (u32, u32)) -> Option<(i32, (f32, f32))> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::query_pointer;

    let (conn, screen) = x11rb::connect(Some(display)).ok()?;
    let setup = conn.setup();
    let screen_info = setup.roots.get(screen)?;
    let root_size = (
        screen_info.width_in_pixels as u32,
        screen_info.height_in_pixels as u32,
    );
    let reply = query_pointer(&conn, screen_info.root)
        .ok()?
        .reply()
        .ok()?;
    let pos = (reply.root_x as f32, reply.root_y as f32);
    log::info!("cursor: X {display} raiz {root_size:?} cursor {pos:?}");
    // Raiz do tamanho do desktop = XWayland espelhando tudo.
    let mut score = if root_size == desktop { 10 } else { 0 };
    // Desempate/aviso: mudou em 25ms = servidor vivo.
    std::thread::sleep(Duration::from_millis(25));
    let moved = query_pointer(&conn, screen_info.root)
        .ok()
        .and_then(|c| c.reply().ok())
        .is_some_and(|r2| r2.root_x != reply.root_x || r2.root_y != reply.root_y);
    if moved {
        score += 5;
    }
    Some((score, pos))
}
