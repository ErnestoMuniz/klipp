use open_gpui::{
    Context, FontWeight, IntoElement, MouseButton, Window, div, prelude::*, px, rgb,
};

use super::icons::{icon, logo};
use super::main_window::MainWindow;
use super::theme;
use crate::core::i18n::{t, t_fmt};
use crate::core::state::UpdateStatus;

/// Titlebar customizada (a janela é frameless — ver `main.rs`).
/// Só as regiões livres arrastam (logo + espaçador central): os botões
/// ficam fora de qualquer handler de drag, senão o compositor rouba o
/// mouse no pressionar e o clique nunca completa.
pub(crate) fn titlebar(
    this: &MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let maximized = window.is_maximized();
    // Pílula de update (só quando há algo acionável): disponível,
    // baixando (com %) ou aplicando. Fora disso, some.
    let (update_status, update_progress, update_lang) = {
        let s = this.shared.lock().unwrap();
        (s.update_status, s.update_progress, s.lang.clone())
    };
    let update_pill = update_pill_state(update_status, update_progress, &update_lang, cx);
    div()
        .flex()
        .items_center()
        .pl_4()
        .pr_2()
        .py_2()
        .bg(theme::titlebar())
        .border_b_1()
        .border_color(theme::border())
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_this, _event, window, _cx| {
                        window.start_window_move();
                    }),
                )
                .child(logo(32.0))
                .child(
                    div()
                        .text_xl()
                        .font_weight(FontWeight::BOLD)
                        // Azul do accent com um passo para cima (gradiente de
                        // texto não existe no GPUI — sólido mais claro).
                        .text_color(theme::accent_hover())
                        .child("Klipp"),
                ),
        )
        .child(
            div()
                .flex_1()
                .self_stretch()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_this, _event, window, _cx| {
                        window.start_window_move();
                    }),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(update_pill)
                .child(window_button("win-min", "lucide-minus", 14.0, false, cx))
                .child(if maximized {
                    window_button("win-restore", "lucide-copy", 12.0, false, cx)
                } else {
                    window_button("win-max", "lucide-square", 12.0, false, cx)
                })
                .child(window_button("win-close", "lucide-x", 14.0, true, cx)),
        )
}

/// Pílula de update na titlebar: só aparece quando há algo acionável
/// (nova versão, download em curso ou aplicando). Retorna elemento vazio
/// nos outros estados para não deslocar os botões da janela.
fn update_pill_state(
    status: UpdateStatus,
    progress: Option<(u64, Option<u64>)>,
    lang: &str,
    cx: &mut Context<MainWindow>,
) -> open_gpui::AnyElement {
    use open_gpui::IntoElement as _;
    let label = match status {        UpdateStatus::Available => t(lang, "update.update_now"),
        UpdateStatus::Downloading => {
            let pct = match progress {
                Some((done, Some(total))) if total > 0 => {
                    format!("{}%", done.saturating_mul(100) / total)
                }
                Some((done, _)) => format!("{} MB", done / 1_048_576),
                None => "…".to_string(),
            };
            t_fmt(lang, "update.downloading", &[("pct", &pct)])
        }
        UpdateStatus::Applying => t(lang, "update.applying"),
        _ => return div().into_any_element(),
    };
    let pill = div()
        .id("titlebar-update")
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .mr_1()
        .bg(theme::accent())
        .rounded(px(6.0))
        .cursor_pointer()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xf2f8ff))
        .hover(|s| s.bg(theme::accent_hover()))
        .child(icon("lucide-download", 13.0, rgb(0xf2f8ff)))
        .child(label);
    if status == UpdateStatus::Available {
        pill.on_click(cx.listener(|this, _e, _w, cx| {
            cx.stop_propagation();
            super::updater::apply_update(&this.shared.clone());
            cx.notify();
        }))
        .into_any_element()
    } else {
        pill.into_any_element()
    }
}

fn window_button(
    id: &'static str,
    asset: &str,
    size_px: f32,
    danger: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let asset = asset.to_string();
    div()
        .id(id)
        .on_click(cx.listener(move |this, _event, window, cx| {
            cx.stop_propagation();
            match id {
                "win-min" => window.minimize_window(),
                "win-max" | "win-restore" => window.zoom_window(),
                _ => this.request_close(window, cx),
            }
        }))
        .size(px(26.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(|s| {
            if danger {
                s.bg(theme::danger_hover())
            } else {
                s.bg(theme::card_hover())
            }
        })
        .text_color(theme::muted())
        .child(icon(&asset, size_px, theme::muted()))
}
