use std::time::Duration;

use open_gpui::{
    div, px, Animation, AnimationExt, Context, FontWeight, IntoElement, ease_out_quint,
    prelude::*,
};

use super::icons::icon;
use super::main_window::MainWindow;
use super::theme;
use super::ui::icon_action_btn;
use crate::core::i18n::t;
use crate::core::state::CursorExt;

/// Dialog de setup do cursor (modal central sobre backdrop, só GNOME).
/// Abre sozinho na largada quando a extensão companion não está ativa;
/// sem ela o pie abre no centro em vez de no cursor.
impl MainWindow {
    pub(crate) fn cursor_prompt_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (open, closing, lang, status) = {
            let s = self.shared.lock().unwrap();
            (
                s.cursor_prompt_open,
                s.cursor_prompt_closing,
                s.lang.clone(),
                s.cursor_ext,
            )
        };
        if !open {
            return div().into_any_element();
        }
        let body: open_gpui::AnyElement = match status {
            CursorExt::Active => div().into_any_element(),
            CursorExt::Missing => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme::muted())
                        .child(t(&lang, "cursor.body")),
                )
                .child(
                    div()
                        .id("cursor-install")
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            cx.stop_propagation();
                            this.install_cursor_extension(cx)
                        }))
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .px_4()
                        .py_2()
                        .bg(theme::accent())
                        .hover(|s| s.bg(theme::accent()))
                        .rounded(px(8.0))
                        .cursor_pointer()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(open_gpui::rgb(0xf2f8ff))
                        .child(icon("lucide-download", 16.0, open_gpui::rgb(0xf2f8ff)))
                        .child(t(&lang, "cursor.install")),
                )
                .into_any_element(),
            CursorExt::Installing => div()
                .text_sm()
                .text_color(theme::muted())
                .child(t(&lang, "cursor.installing"))
                .into_any_element(),
            CursorExt::NeedsLogin => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme::muted())
                        .child(t(&lang, "cursor.login")),
                )
                .child(ok_button(&t(&lang, "cursor.ok"), cx))
                .into_any_element(),
            CursorExt::Disabled => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme::muted())
                        .child(t(&lang, "cursor.banner_disabled")),
                )
                .child(
                    div()
                        .id("cursor-enable")
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            cx.stop_propagation();
                            this.enable_cursor_extension(cx)
                        }))
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .px_4()
                        .py_2()
                        .bg(theme::accent())
                        .hover(|s| s.bg(theme::accent()))
                        .rounded(px(8.0))
                        .cursor_pointer()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(open_gpui::rgb(0xf2f8ff))
                        .child(icon("lucide-download", 16.0, open_gpui::rgb(0xf2f8ff)))
                        .child(t(&lang, "cursor.enable")),
                )
                .into_any_element(),
            CursorExt::Unknown => div()
                .text_sm()
                .text_color(theme::muted())
                .child(t(&lang, "cursor.checking"))
                .into_any_element(),
        };
        let card = div()
            .relative()
            .w(px(440.0))
            .flex()
            .flex_col()
            .items_center()
            .gap_4()
            .p_7()
            .bg(theme::panel())
            .border_1()
            .border_color(theme::border())
            .rounded(px(12.0))
            .id("cursor-card")
            .on_click(cx.listener(|_this, _e, _w, cx| {
                cx.stop_propagation();
            }))
            .child(
                div()
                    .absolute()
                    .top(px(12.0))
                    .right(px(12.0))
                    .child(
                        icon_action_btn("lucide-x", theme::muted(), 16.0)
                            .id("cursor-close")
                            .on_click(cx.listener(|this, _e, _w, cx| this.close_cursor_prompt(cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(t(&lang, "cursor.title")),
            )
            .child(body)
            .with_animation(
                if closing {
                    "cursor-card-out"
                } else {
                    "cursor-card"
                },
                Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint()),
                move |el, delta| {
                    let d = if closing { 1.0 - delta } else { delta };
                    el.opacity(d)
                },
            );
        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .child(
                div()
                    .absolute()
                    .top(px(0.0))
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .bg(open_gpui::rgba(0x00000000))
                    .id("cursor-backdrop")
                    .on_click(cx.listener(|this, _e, _w, cx| {
                            cx.stop_propagation();
                            this.close_cursor_prompt(cx)
                        }),
                    )
                    .with_animation(
                        if closing {
                            "cursor-backdrop-out"
                        } else {
                            "cursor-backdrop"
                        },
                        Animation::new(Duration::from_millis(180)),
                        move |el, delta| {
                            let d = if closing { 1.0 - delta } else { delta };
                            let a = (0x73 as f32 * d) as u32;
                            el.bg(open_gpui::rgba(a))
                        },
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(0.0))
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .child(card),
            )
            .into_any_element()
    }

    pub(crate) fn close_cursor_prompt(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.cursor_prompt_open && !s.cursor_prompt_closing {
            // Mantém montado: o fade reverso toca, o tick desmonta.
            s.cursor_prompt_closing = true;
            s.cursor_prompt_anim_start = super::format::now_ms();
            s.bump();
            cx.notify();
        }
    }
}

fn ok_button(label: &str, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div()
        .id("cursor-ok")
        .on_click(cx.listener(|this, _e, _w, cx| this.close_cursor_prompt(cx)))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .px_4()
        .py_2()
        .bg(theme::card())
        .border_1()
        .border_color(theme::border())
        .hover(|s| s.bg(theme::card_hover()))
        .rounded(px(8.0))
        .cursor_pointer()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.to_string())
}
