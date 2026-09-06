use std::collections::HashSet;
use std::time::Duration;

use gpui_animation::animation::TransitionExt;
use gpui_animation::transition::general::Linear;
use open_gpui::{Context, ElementId, FontWeight, IntoElement, div, prelude::*, px};

use super::format::{format_duration, rgb_dark, sound_label};
use super::icons::icon;
use super::main_window::MainWindow;
use super::theme;
use super::ui::icon_action_btn;
use crate::core::i18n::{t, t_fmt};
use crate::core::state::Sound;

pub(crate) fn empty_library(lang: &str, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .mt_2()
        .bg(theme::panel())
        .border_1()
        .border_dashed()
        .border_color(theme::dashed())
        .rounded(px(14.0))
        .py_16()
        .child(
            div()
                .p_4()
                .bg(theme::card())
                .border_1()
                .border_color(theme::border())
                .rounded(px(12.0))
                .text_color(theme::accent())
                .child(icon("lucide-music", 24.0, theme::accent())),
        )
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child(t(lang, "empty.title")),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme::muted())
                .child(t(lang, "empty.body")),
        )
        .child(
            div()
                .id("add-sounds")
                .on_click(cx.listener(|this, _e, _w, cx| this.pick_files(cx)))
                .mt_2()
                .px_5()
                .py_2()
                .flex()
                .items_center()
                .gap_2()
                .bg(theme::accent())
                .rounded(px(8.0))
                .cursor_pointer()
                .hover(|s| s.bg(theme::accent_hover()))
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb_dark())
                .child(icon("lucide-plus", 14.0, rgb_dark()))
                .child(t(lang, "empty.add")),
        )
}

pub(crate) fn drop_overlay(lang: &str, active: bool) -> impl IntoElement {
    if !active {
        return div().into_any_element();
    }
    div()
        .absolute()
        .top(px(8.0))
        .left(px(8.0))
        .right(px(8.0))
        .bottom(px(8.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(open_gpui::rgba(0x00000066))
        .border_1()
        .border_dashed()
        .border_color(theme::accent())
        .rounded(px(14.0))
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::accent())
                .child(t(lang, "drop.label")),
        )
        .into_any_element()
}

pub(crate) fn no_results(
    lang: &str,
    query: &str,
    _only_fav: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .mt_2()
        .py_16()
        .child(
            div()
                .p_4()
                .bg(theme::card())
                .border_1()
                .border_color(theme::border())
                .rounded(px(12.0))
                .child(icon("lucide-search", 24.0, theme::muted())),
        )
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .child(t(lang, "empty.none_title")),
        )
        .child(div().text_sm().text_color(theme::muted()).child(t_fmt(
            lang,
            "empty.none_body",
            &[("query", query)],
        )))
        .child(
            div()
                .id("clear-search")
                .on_click(cx.listener(|this, _e, _w, cx| this.clear_search(cx)))
                .mt_1()
                .px_4()
                .py_2()
                .bg(theme::card())
                .border_1()
                .border_color(theme::border())
                .rounded(px(8.0))
                .cursor_pointer()
                .hover(|s| s.bg(theme::card_hover()))
                .text_sm()
                .child(t(lang, "toolbar.clear")),
        )
}

#[derive(Clone, Copy)]
pub(crate) enum CardAction {
    Favorite,
    Delete,
    Edit,
}

/// Emoji do pad como imagem rasterizada (swash); "♪" padrão vai direto
/// como texto do sistema, e emoji ainda em raster mostra caixa vazia.
/// Reusado no pie do overlay.
pub(crate) fn pad_emoji(
    emoji: &str,
    size: open_gpui::Pixels,
    shared: &std::sync::Arc<std::sync::Mutex<crate::core::state::Shared>>,
) -> open_gpui::AnyElement {
    if emoji == crate::backend::sounds::DEFAULT_EMOJI {
        return div().text_xl().child(emoji.to_string()).into_any_element();
    }
    match super::emoji::image(emoji, shared) {
        Some(image) => open_gpui::img(image).w(size).h(size).into_any_element(),
        None => div().w(size).h(size).into_any_element(),
    }
}

impl MainWindow {
    pub(crate) fn sound_cards(
        &self,
        sounds: &[Sound],
        unfavorited: &HashSet<String>,
        playing: Option<&String>,
        compact: bool,
        lang: &str,
        modal_open: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Com drawer/about abertos, o backdrop cobre os cards: sem hover visuals.
        let hovered_card = if modal_open { None } else { self.hovered_card };
        if compact {
            return self
                .compact_rows(sounds, unfavorited, playing, lang, hovered_card, cx)
                .into_any_element();
        }
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_4()
            .pt_2()
            .children(
                sounds
                    .iter()
                    .enumerate()
                    .map(|(i, sound)| {
                        let is_playing = playing == Some(&sound.name);
                        let fav = !unfavorited.contains(&sound.name);
                        let hovered = hovered_card == Some(i);
                        let title = sound_label(sound);
                        let play_name = sound.name.clone();
                        let fav_name = sound.name.clone();
                        let del_name = sound.name.clone();
                        let edit_name = sound.name.clone();
                        let card_bg = if fav {
                            theme::card()
                        } else {
                            theme::card_inner()
                        };
                        let title_el: open_gpui::AnyElement = div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .text_color(if fav { theme::text() } else { theme::muted() })
                            .child(title)
                            .into_any_element();
                        let actions: open_gpui::AnyElement = div()
                            .id(format!("card-{i}-actions"))
                            .absolute()
                            .top(px(6.0))
                            .right(px(6.0))
                            .flex()
                            .justify_center()
                            .opacity(if hovered { 1.0 } else { 0.0 })
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_1()
                                    .p_1()
                                    .bg(theme::panel())
                                    .border_1()
                                    .border_color(theme::border())
                                    .rounded(px(8.0))
                                    .child(self.card_action(
                                        &format!("card-{i}-fav"),
                                        if fav {
                                            "lucide-star-filled"
                                        } else {
                                            "lucide-star"
                                        },
                                        if fav { theme::accent() } else { theme::muted() },
                                        fav_name,
                                        CardAction::Favorite,
                                        cx,
                                    ))
                                    .child(self.card_action(
                                        &format!("card-{i}-del"),
                                        "lucide-trash-2",
                                        theme::danger_hover(),
                                        del_name,
                                        CardAction::Delete,
                                        cx,
                                    ))
                                    .child(self.card_action(
                                        &format!("card-{i}-ren"),
                                        "lucide-pencil",
                                        theme::muted(),
                                        edit_name,
                                        CardAction::Edit,
                                        cx,
                                    )),
                            )
                            .with_transition(format!("card-actions-{}", sound.name))
                            .transition_when_else(
                                hovered,
                                Duration::from_millis(100),
                                Linear,
                                |s| s.opacity(1.0),
                                |s| s.opacity(0.0),
                            )
                            .into_any_element();
                        div()
                            .id(ElementId::from(i))
                            .on_hover(cx.listener(move |this, over: &bool, _window, cx| {
                                this.set_hovered(if *over { Some(i) } else { None }, cx)
                            }))
                            .on_click(cx.listener(move |this, _event, window, cx| {
                                this.play_sound(play_name.clone(), window, cx)
                            }))
                            .w(px(150.0))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .bg(card_bg)
                            .border_1()
                            .border_color(theme::border())
                            .rounded(px(12.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::card_hover()))
                            .relative()
                            .child(
                                div()
                                    .h(px(56.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(theme::card_inner())
                                    .border_1()
                                    .border_color(if is_playing {
                                        theme::accent()
                                    } else {
                                        theme::border()
                                    })
                                    .rounded(px(8.0))
                                    .child(pad_emoji(&sound.emoji, px(30.0), &self.shared)),
                            )
                            .child(title_el)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_xs()
                                    .text_color(theme::muted())
                                    .child(if is_playing {
                                        t(lang, "pad.playing")
                                    } else {
                                        t(lang, "pad.play")
                                    })
                                    .child(format_duration(sound.duration_secs)),
                            )
                            .child(actions)
                            .with_transition(format!("audio-card-{}", sound.name))
                            .transition_when_else(
                                hovered,
                                Duration::from_millis(100),
                                Linear,
                                |s| s.border_color(theme::accent()),
                                |s| s.border_color(theme::border()),
                            )
                    })
                    .collect::<Vec<_>>(),
            )
            .into_any_element()
    }

    /// Modo compacto: mini-cards horizontais em grade (não lista).
    fn compact_rows(
        &self,
        sounds: &[Sound],
        unfavorited: &HashSet<String>,
        playing: Option<&String>,
        _lang: &str,
        hovered_card: Option<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_3()
            .pt_2()
            .children(
                sounds
                    .iter()
                    .enumerate()
                    .map(|(i, sound)| {
                        let is_playing = playing == Some(&sound.name);
                        let fav = !unfavorited.contains(&sound.name);
                        let hovered = hovered_card == Some(i);
                        let play_name = sound.name.clone();
                        div()
                            .id(ElementId::from(i))
                            .on_hover(cx.listener(move |this, over: &bool, _window, cx| {
                                this.set_hovered(if *over { Some(i) } else { None }, cx)
                            }))
                            .on_click(cx.listener(move |this, _event, window, cx| {
                                this.play_sound(play_name.clone(), window, cx)
                            }))
                            .w(px(170.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .p_2()
                            .bg(if fav {
                                theme::card()
                            } else {
                                theme::card_inner()
                            })
                            .border_1()
                            .border_color(if is_playing || hovered {
                                theme::accent()
                            } else {
                                theme::border()
                            })
                            .rounded(px(10.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::card_hover()))
                            .child(
                                div()
                                    .size(px(40.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .flex_shrink_0()
                                    .bg(theme::card_inner())
                                    .border_1()
                                    .border_color(theme::border())
                                    .rounded(px(8.0))
                                    .child(pad_emoji(&sound.emoji, px(22.0), &self.shared)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap_0()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .truncate()
                                            .text_color(if fav {
                                                theme::text()
                                            } else {
                                                theme::muted()
                                            })
                                            .child(sound_label(sound)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme::muted())
                                            .child(format_duration(sound.duration_secs)),
                                    ),
                            )
                            .with_transition(format!("audio-card-compact-{}", sound.name))
                            .transition_when_else(
                                hovered || is_playing,
                                Duration::from_millis(100),
                                Linear,
                                |s| s.border_color(theme::accent()),
                                |s| s.border_color(theme::border()),
                            )
                    })
                    .collect::<Vec<_>>(),
            )
            .into_any_element()
    }

    fn card_action(
        &self,
        id: &str,
        asset: &str,
        color: open_gpui::Rgba,
        name: String,
        action: CardAction,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        icon_action_btn(asset, color, 14.0)
            .id(id.to_string())
            .on_click(cx.listener(move |this, _event, window, cx| {
                cx.stop_propagation();
                match action {
                    CardAction::Favorite => this.toggle_favorite(name.clone(), cx),
                    CardAction::Delete => this.delete_sound(name.clone(), cx),
                    CardAction::Edit => this.open_editor(name.clone(), window, cx),
                }
            }))
    }
}
