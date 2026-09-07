use open_gpui::{Context, FontWeight, IntoElement, MouseButton, Window, div, prelude::*, px};

use super::icons::{icon, logo};
use super::main_window::MainWindow;
use super::theme;

/// Titlebar customizada (a janela é frameless — ver `main.rs`).
/// Só as regiões livres arrastam (logo + espaçador central): os botões
/// ficam fora de qualquer handler de drag, senão o compositor rouba o
/// mouse no pressionar e o clique nunca completa.
pub(crate) fn titlebar(window: &mut Window, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let maximized = window.is_maximized();
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
                .child(window_button("win-min", "lucide-minus", 14.0, false, cx))
                .child(if maximized {
                    window_button("win-restore", "lucide-copy", 12.0, false, cx)
                } else {
                    window_button("win-max", "lucide-square", 12.0, false, cx)
                })
                .child(window_button("win-close", "lucide-x", 14.0, true, cx)),
        )
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
