use std::time::Duration;

use open_gpui::{
    div, px, Animation, AnimationExt, Context, Div, IntoElement, Rgba,
    ease_out_quint, prelude::*,
};

use super::format::caret_on;
use super::icons::icon;
use super::main_window::MainWindow;
use super::theme;
use crate::core::i18n::t;

/// Base de botão-ícone clicável: padding, cantos, cursor e hover neutro.
/// O chamador encadeia `.id()` + `.on_mouse_down()`.
pub(crate) fn icon_action_btn(asset: &str, color: Rgba, size_px: f32) -> Div {
    div()
        .p_2()
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(|s| s.bg(theme::card_hover()))
        .child(icon(asset, size_px, color))
}

/// Tecla estilizada (`kbd`) para os banners.
pub(crate) fn kbd(label: &str) -> impl IntoElement {
    div()
        .px_2()
        .mx_1()
        .bg(theme::card())
        .border_1()
        .border_color(theme::border())
        .rounded(px(6.0))
        .text_xs()
        .child(label.to_string())
}

/// Caret de 2px que pisca enquanto `visible` (campo focado).
/// `off` é a cor de fundo do campo (evita deslocar o layout ao apagar).
pub(crate) fn caret(visible: bool, off: Rgba, h_px: f32) -> Div {
    let bg = if visible && caret_on() {
        theme::accent()
    } else {
        off
    };
    div().w(px(2.0)).h(px(h_px)).bg(bg)
}

/// Fade-in para conteúdos de dropdown (mic, ordenação).
/// Terminal: aplicar por último no `Div`, antes de `deferred`.
pub(crate) fn fade_in(el: Div, id: String, ms: u64) -> impl IntoElement {
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(ms)).with_easing(ease_out_quint()),
        |el, delta| el.opacity(delta),
    )
}

/// Faixa com a dica do atalho global. Some via `✕` ou pela lâmpada.
pub(crate) fn hint_banner(
    lang: &str,
    shortcut: &str,
    show: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    if !show {
        return div().into_any_element();
    }
    div()
        .mx_4()
        .mt_1()
        .mb_2()
        .px_4()
        .py_2()
        .flex()
        .items_center()
        .gap_2()
        .bg(theme::hint_bg())
        .border_1()
        .border_dashed()
        .border_color(theme::hint_border())
        .rounded(px(10.0))
        .child(icon("lucide-command", 14.0, theme::accent()))
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .gap_1()
                .text_sm()
                .child(t(lang, "hint.prefix"))
                .child(kbd(shortcut))
                .child(t(lang, "hint.middle"))
                .child(kbd("Esc"))
                .child(t(lang, "hint.suffix")),
        )
        .child(
            icon_action_btn("lucide-x", theme::muted(), 12.0)
                .id("hint-close")
                .on_click(cx.listener(|this, _e, _w, cx| this.dismiss_hint(cx))),
        )
        .into_any_element()
}

/// Faixa persistente do cursor (só GNOME, só sem extensão ativa): sem X
/// de propósito — some sozinha quando a extensão ativa. No `Disabled` tem
/// botão Habilitar; no `NeedsLogin`, a instrução de sair/entrar.
pub(crate) fn cursor_banner(
    lang: &str,
    status: crate::core::state::CursorExt,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    use crate::core::state::CursorExt as C;
    if !crate::backend::gnome_shortcuts::is_desktop_gnome() {
        return div().into_any_element();
    }
    let enable: Option<open_gpui::AnyElement> = match status {
        C::Disabled => Some(
            div()
                .id("cursor-banner-enable")
                .on_click(cx.listener(|this, _e, _w, cx| {
                    cx.stop_propagation();
                    this.enable_cursor_extension(cx)
                }))
                .px_3()
                .py_1()
                .bg(theme::accent())
                .hover(|s| s.bg(theme::accent()))
                .rounded(px(6.0))
                .cursor_pointer()
                .text_xs()
                .font_weight(open_gpui::FontWeight::SEMIBOLD)
                .text_color(open_gpui::rgb(0xf2f8ff))
                .child(t(lang, "cursor.enable"))
                .into_any_element(),
        ),
        _ => None,
    };
    let text_key = match status {
        C::NeedsLogin => Some("cursor.banner_login"),
        C::Disabled => Some("cursor.banner_disabled"),
        _ => None,
    };
    let Some(text_key) = text_key else {
        return div().into_any_element();
    };
    let mut row = div()
        .flex_1()
        .flex()
        .items_center()
        .gap_2()
        .text_sm()
        .child(t(lang, text_key));
    if let Some(enable) = enable {
        row = row.child(enable);
    }
    div()
        .mx_4()
        .mt_1()
        .mb_2()
        .px_4()
        .py_2()
        .flex()
        .items_center()
        .gap_2()
        .bg(theme::hint_bg())
        .border_1()
        .border_dashed()
        .border_color(theme::hint_border())
        .rounded(px(10.0))
        .child(icon("lucide-command", 14.0, theme::accent()))
        .child(row)
        .into_any_element()
}

pub(crate) fn status_banner(last_error: Option<String>) -> impl IntoElement {
    match last_error {
        Some(err) => div()
            .mx_4()
            .mb_2()
            .px_4()
            .py_2()
            .bg(theme::danger())
            .rounded(px(10.0))
            .text_sm()
            .child(err)
            .into_any_element(),
        None => div().into_any_element(),
    }
}
