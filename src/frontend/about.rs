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
use crate::core::state::UpdateStatus;

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
            .child(self.update_block(&lang, cx))
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

    /// Bloco de atualização no sobre: status + "verificar" + (se houver
    /// release nova) botão de ação — atualiza e reinicia no AppImage,
    /// abre a página da release fora dele (dev, tarball...).
    fn update_block(&self, lang: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let (status, version, error, progress) = {
            let s = self.shared.lock().unwrap();
            (
                s.update_status,
                s.update_version.clone(),
                s.update_error.clone(),
                s.update_progress,
            )
        };
        let is_appimage = crate::backend::updater::appimage_path().is_some();
        let (status_text, status_color) = match status {
            UpdateStatus::Idle => (String::new(), theme::muted()),
            UpdateStatus::Checking => (t(lang, "update.checking"), theme::muted()),
            UpdateStatus::UpToDate => (t(lang, "update.up_to_date"), theme::muted()),
            UpdateStatus::Available => (
                t_fmt(
                    lang,
                    "update.available",
                    &[("version", version.as_deref().unwrap_or("?"))],
                ),
                theme::accent_hover(),
            ),
            UpdateStatus::Downloading => {
                let pct = match progress {
                    Some((done, Some(total))) if total > 0 => {
                        format!("{}%", done.saturating_mul(100) / total)
                    }
                    Some((done, _)) => format!("{} MB", done / 1_048_576),
                    None => "…".to_string(),
                };
                (
                    t_fmt(lang, "update.downloading", &[("pct", &pct)]),
                    theme::accent_hover(),
                )
            }
            UpdateStatus::Applying => (t(lang, "update.applying"), theme::accent_hover()),
            UpdateStatus::Error => (
                t_fmt(
                    lang,
                    "update.error",
                    &[("msg", error.as_deref().unwrap_or("?"))],
                ),
                theme::danger_hover(),
            ),
        };
        let busy = matches!(
            status,
            UpdateStatus::Checking | UpdateStatus::Downloading | UpdateStatus::Applying
        );
        let mut block = div().w_full().flex().flex_col().gap_2().child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(status_color)
                .child(status_text),
        );
        // Ação principal quando há release nova (antes do "verificar").
        if status == UpdateStatus::Available
            && let Some(ver) = version
        {
            let label = if is_appimage {
                t(lang, "update.update_now")
            } else {
                t_fmt(lang, "update.get", &[("version", &ver)])
            };
            block = block.child(
                div()
                    .id("about-update-apply")
                    .on_click(cx.listener(|this, _e, _w, cx| {
                        cx.stop_propagation();
                        super::updater::apply_update(&this.shared.clone());
                        cx.notify();
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
                    .hover(|s| s.bg(theme::accent_hover()))
                    .rounded(px(8.0))
                    .cursor_pointer()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(open_gpui::rgb(0xf2f8ff))
                    .child(icon("lucide-download", 16.0, open_gpui::rgb(0xf2f8ff)))
                    .child(label),
            );
        }
        block
            .child(
                div()
                    .id("about-update-check")
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        cx.stop_propagation();
                        if busy {
                            return;
                        }
                        super::updater::check_updates(&this.shared.clone());
                        cx.notify();
                    }))
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
                    .child(t(lang, "update.check")),
            )
            .into_any_element()
    }

    pub(crate) fn open_about(&self, cx: &mut Context<Self>) {        {
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
