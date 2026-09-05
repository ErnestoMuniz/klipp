use std::time::Duration;

use open_gpui::{
    div, px, Animation, AnimationExt, Context, FontWeight, IntoElement, ease_out_quint,
    prelude::*,
};

use super::icons::{icon, logo};
use super::main_window::MainWindow;
use super::theme;
use super::ui::icon_action_btn;
use crate::core::i18n::{t, t_fmt};

const REPOSITORY_URL: &str = "https://github.com/ErnestoMuniz/klipp";

/// Dialog "About Klipp" (modal central sobre backdrop).
impl MainWindow {
    pub(crate) fn about_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (open, closing, lang) = {
            let s = self.shared.lock().unwrap();
            (s.about_open, s.about_closing, s.lang.clone())
        };
        if !open {
            return div().into_any_element();
        }
        // Fade de abertura/fechamento: backdrop escurece e o card aparece
        // em opacity (mesmo padrão dos drawers de settings/browse).
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
            .id("about-card")
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
                            .id("about-close")
                            .on_click(cx.listener(|this, _e, _w, cx| this.close_about(cx)),
                            ),
                    ),
            )
            .child(logo(56.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Klipp"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme::muted())
                            .child(t_fmt(
                                &lang,
                                "about.version",
                                &[("version", env!("CARGO_PKG_VERSION"))],
                            )),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme::muted())
                    .child(t(&lang, "about.description")),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .border_t_1()
                    .border_b_1()
                    .border_color(theme::border())
                    .py_4()
                    .child(about_row(&t(&lang, "about.creator"), "Ernesto Muniz"))
                    .child(about_row(&t(&lang, "about.platform"), "Linux"))
                    .child(about_row(&t(&lang, "about.tech"), "Rust · GPUI")),
            )
            .child(
                div()
                    .id("about-github")
                    .on_click(cx.listener(|_this, _e, _w, _cx| {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(REPOSITORY_URL)
                                .spawn();
                        }),
                    )
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .gap_2()
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
                    .child(icon("logo-github", 18.0, theme::text()))
                    .child(t(&lang, "about.github")),
            )
            .with_animation(
                if closing {
                    "about-card-out"
                } else {
                    "about-card"
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
                    .id("about-backdrop")
                    .on_click(cx.listener(|this, _e, _w, cx| {
                            cx.stop_propagation();
                            this.close_about(cx)
                        }),
                    )
                    .with_animation(
                        if closing {
                            "about-backdrop-out"
                        } else {
                            "about-backdrop"
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

    pub(crate) fn open_about(&self, cx: &mut Context<Self>) {
        {
            let mut s = self.shared.lock().unwrap();
            s.about_open = true;
            s.about_closing = false;
            s.bump();
        }
        cx.notify();
    }

    pub(crate) fn close_about(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.about_open && !s.about_closing {
            // Mantém montado: o fade reverso toca, o tick desmonta.
            s.about_closing = true;
            s.about_anim_start = super::format::now_ms();
            s.bump();
            cx.notify();
        }
    }
}

fn about_row(label: &str, value: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_4()
        .text_sm()
        .child(
            div()
                .w(px(110.0))
                .text_color(theme::muted())
                .child(label.to_string()),
        )
        .child(
            div()
                .flex_1()
                .font_weight(FontWeight::SEMIBOLD)
                .child(value.to_string()),
        )
}
