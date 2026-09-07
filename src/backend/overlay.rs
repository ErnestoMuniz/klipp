use std::sync::{Arc, Mutex};

use crate::core::state::Shared;

/// Abre o overlay no cursor, no mesmo estado do atalho global do portal
/// (ver `backend::shortcuts`): âncora pendente + fallback central até a
/// posição exata chegar (thread dedicada ou primeiro mouse_move).
pub fn open(shared: &Arc<Mutex<Shared>>) {
    {
        let mut s = shared.lock().unwrap();
        // Com áudio tocando, o centro (stop) já nasce selecionado.
        let stopping = s.playing.is_some();
        s.overlay_active = true;
        s.overlay_fading = false;
        s.overlay_seq = s.overlay_seq.wrapping_add(1);
        s.overlay_anchor = None;
        s.anchor_needs_confirm = true;
        s.pie_hovered = None;
        s.center_hovered = stopping;
        s.confirm_request = false;
        s.bump();
    }
    resolve_cursor(shared);
}

/// Alterna: aberto → confirma a seleção (equivale a soltar o atalho);
/// fechado → abre. Usado pelo `--toggle-overlay` (atalho custom do DE).
pub fn toggle(shared: &Arc<Mutex<Shared>>) {
    let active = shared
        .lock()
        .map(|s| s.overlay_active && !s.overlay_fading)
        .unwrap_or(false);
    if active {
        if let Ok(mut s) = shared.lock() {
            s.confirm_request = true;
            s.bump();
        }
    } else {
        open(shared);
    }
}

/// Resolve a posição exata em thread dedicada: `get_position()` é
/// bloqueante e não pode travar a UI (nem o listener de IPC).
fn resolve_cursor(shared: &Arc<Mutex<Shared>>) {
    let shared_bg = shared.clone();
    std::thread::Builder::new()
        .name("klipp-cursor".into())
        .spawn(move || {
            match mouse_coords::get_position() {
                Ok(pos) => {
                    log::info!("overlay cursor: ({}, {})", pos.x, pos.y);
                    if let Ok(mut s) = shared_bg.lock() {
                        if s.overlay_active {
                            s.overlay_anchor = Some((pos.x as f32, pos.y as f32));
                            s.anchor_needs_confirm = false;
                            s.bump();
                        }
                    }
                }
                Err(err) => {
                    log::info!("overlay cursor indisponível: {err}");
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::Settings;

    fn shared() -> Arc<Mutex<Shared>> {
        Arc::new(Mutex::new(Shared::new(&Settings::default())))
    }

    #[test]
    fn toggle_com_overlay_aberto_pede_confirmacao() {
        let s = shared();
        {
            let mut g = s.lock().unwrap();
            g.overlay_active = true;
            g.pie_hovered = Some(1);
        }
        toggle(&s);
        let g = s.lock().unwrap();
        assert!(g.confirm_request);
        // Sem thread de cursor nesse caminho: âncora segue intocada.
        assert!(g.overlay_anchor.is_none());
    }

    #[test]
    fn toggle_com_overlay_fechado_ativa() {
        let s = shared();
        // Sem display: `get_position` falha rápido na thread e a âncora
        // segue pendente até o primeiro mouse_move (mesmo fluxo do portal).
        let vars = ["XDG_SESSION_TYPE", "WAYLAND_DISPLAY", "DISPLAY"];
        let saved: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|k| (k.to_string(), std::env::var(k).ok()))
            .collect();
        for k in &vars {
            unsafe { std::env::remove_var(k) };
        }
        toggle(&s);
        let g = s.lock().unwrap();
        assert!(g.overlay_active);
        assert!(!g.overlay_fading);
        assert!(g.anchor_needs_confirm);
        assert!(g.overlay_anchor.is_none());
        drop(g);
        for (k, v) in saved {
            unsafe {
                match v {
                    Some(val) => std::env::set_var(&k, val),
                    None => std::env::remove_var(&k),
                }
            }
        }
    }
}
