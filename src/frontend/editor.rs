use std::time::Duration;

use open_gpui::{
    Animation, AnimationExt, Context, FontWeight, IntoElement, MouseButton, Window, div,
    ease_out_quint, prelude::*, px,
};

use super::emoji;
use super::format::{rgb_dark, sound_label};
use super::icons::icon;
use super::main_window::{MainWindow, TextField};
use super::theme;
use super::ui::{caret, icon_action_btn};
use crate::backend;
use crate::core::i18n::t;

/// Dialog modal "Edit sound" (igual ao app Electron, sem a parte do switch
/// do quick picker): nome de exibição + picker de emoji, Cancel/Save.
/// Abre no lápis do hover dos cards.
impl MainWindow {
    pub(crate) fn editor_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (
            open,
            closing,
            lang,
            name,
            display,
            emoji_pick,
            group,
            query,
            name_focused,
            search_focused,
            name_selected,
            search_selected,
        ) = {
            let s = self.shared.lock().unwrap();
            (
                s.editor_open,
                s.editor_closing,
                s.lang.clone(),
                s.editor_name.clone(),
                s.editor_display.clone(),
                s.editor_emoji.clone(),
                s.editor_group.clone(),
                s.editor_emoji_query.clone(),
                s.editor_name_focused,
                s.editor_search_focused,
                s.editor_name_selected,
                s.editor_search_selected,
            )
        };
        if !open {
            return div().into_any_element();
        }
        // Fade de abertura/fechamento: backdrop escurece e o painel aparece
        // em opacity (mesmo padrão do dialog "About").
        let panel = div()
                    .w(px(560.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .bg(theme::panel())
                    .border_1()
                    .border_color(theme::border())
                    .rounded(px(12.0))
                    .id("editor-panel")
                    .on_click(cx.listener(|_this, _e, _w, cx| {
                        cx.stop_propagation();
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap_4()
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(t(&lang, "editor.title")),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .truncate()
                                            .text_color(theme::muted())
                                            .child(name),
                                    ),
                            )
                            .child(
                                icon_action_btn("lucide-x", theme::muted(), 16.0)
                                    .id("editor-close")
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.close_editor(cx)
                                    })),
                            ),
                    )
                    .child(self.editor_name_field(
                        &lang,
                        &display,
                        name_focused,
                        name_selected,
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::muted())
                                    .child(t(&lang, "editor.emoji").to_uppercase()),
                            )
                            .child(self.editor_search_field(
                                &lang,
                                &query,
                                search_focused,
                                search_selected,
                                cx,
                            )),
                    )
                    .child(editor_tabs(&group, cx))
                    .child(editor_grid(
                        &lang,
                        &query,
                        &group,
                        &emoji_pick,
                        &self.shared,
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .id("editor-cancel")
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        cx.stop_propagation();
                                        this.close_editor(cx)
                                    }))
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
                                    .child(t(&lang, "editor.cancel")),
                            )
                            .child(
                                div()
                                    .id("editor-save")
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        cx.stop_propagation();
                                        this.save_editor(cx)
                                    }))
                                    .px_4()
                                    .py_2()
                                    .bg(theme::accent())
                                    .hover(|s| s.bg(theme::accent_hover()))
                                    .rounded(px(8.0))
                                    .cursor_pointer()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb_dark())
                                    .child(t(&lang, "editor.save")),
                            ),
                    )
                    .with_animation(
                        if closing {
                            "editor-panel-out"
                        } else {
                            "editor-panel"
                        },
                        Animation::new(Duration::from_millis(180))
                            .with_easing(ease_out_quint()),
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
                    .id("editor-backdrop")
                    .on_click(cx.listener(|this, _e, _w, cx| {
                        cx.stop_propagation();
                        this.close_editor(cx)
                    }))
                    .with_animation(
                        if closing {
                            "editor-backdrop-out"
                        } else {
                            "editor-backdrop"
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
                    .p_8()
                    .child(panel),
            )
            .into_any_element()
    }

    fn editor_name_field(
        &self,
        lang: &str,
        display: &str,
        focused: bool,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let text: open_gpui::AnyElement = if display.is_empty() {
            div()
                .text_sm()
                .truncate()
                .text_color(theme::muted())
                .child(t(lang, "editor.name_ph"))
                .into_any_element()
        } else {
            div()
                .text_sm()
                .truncate()
                .bg(if selected {
                    theme::sel_bg()
                } else {
                    open_gpui::rgba(0x00000000)
                })
                .rounded(px(4.0))
                .child(display.to_string())
                .into_any_element()
        };
        let caret_on = focused;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(t(lang, "editor.name")),
            )
            .child(
                div()
                    .id("editor-name-box")
                    .track_focus(&self.editor_focus)
                    .on_click(cx.listener(|this, _e, window, cx| {
                        cx.stop_propagation();
                        this.focus_editor_name(window, cx)
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(
                            |this, event: &open_gpui::MouseDownEvent, window, cx| {
                                cx.stop_propagation();
                                this.input_mouse_down(
                                    TextField::EditorName,
                                    f32::from(event.position.x),
                                    event.click_count,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, event: &open_gpui::MouseUpEvent, _window, cx| {
                            this.input_mouse_up(
                                TextField::EditorName,
                                f32::from(event.position.x),
                                cx,
                            )
                        }),
                    )
                    .on_key_down(cx.listener(|this, event, window, cx| {
                        this.on_editor_name_key(event, window, cx)
                    }))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .bg(theme::card_inner())
                    .border_1()
                    .border_color(if focused {
                        theme::accent()
                    } else {
                        theme::border()
                    })
                    .rounded(px(8.0))
                    .cursor_text()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_row()
                            .items_center()
                            .child(if display.is_empty() {
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .child(caret(caret_on, theme::card_inner(), 16.0))
                                    .child(text)
                                    .into_any_element()
                            } else {
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .child(text)
                                    .child(caret(caret_on, theme::card_inner(), 16.0))
                                    .into_any_element()
                            }),
                    ),
            )
    }

    fn editor_search_field(
        &self,
        lang: &str,
        query: &str,
        focused: bool,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let text: open_gpui::AnyElement = if query.is_empty() {
            div()
                .text_sm()
                .truncate()
                .text_color(theme::muted())
                .child(t(lang, "editor.search_ph"))
                .into_any_element()
        } else {
            div()
                .text_sm()
                .truncate()
                .bg(if selected {
                    theme::sel_bg()
                } else {
                    open_gpui::rgba(0x00000000)
                })
                .rounded(px(4.0))
                .child(query.to_string())
                .into_any_element()
        };
        let clear: open_gpui::AnyElement = if query.is_empty() {
            div().into_any_element()
        } else {
            icon_action_btn("lucide-x", theme::muted(), 12.0)
                .id("editor-emoji-clear")
                .on_click(cx.listener(|this, _e, _w, cx| this.clear_editor_search(cx)))
                .into_any_element()
        };
        // Vazio: caret antes do placeholder; com texto: depois (igual ao name).
        let text_row: open_gpui::AnyElement = if query.is_empty() {
            div()
                .flex_1()
                .flex()
                .flex_row()
                .items_center()
                .overflow_hidden()
                .child(caret(focused, theme::card_inner(), 14.0))
                .child(text)
                .into_any_element()
        } else {
            div()
                .flex_1()
                .flex()
                .flex_row()
                .items_center()
                .overflow_hidden()
                .child(text)
                .child(caret(focused, theme::card_inner(), 14.0))
                .into_any_element()
        };
        div()
            .id("editor-emoji-box")
            .track_focus(&self.editor_search_focus)
            .on_click(cx.listener(|this, _e, window, cx| {
                cx.stop_propagation();
                this.focus_editor_search(window, cx)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(
                    |this, event: &open_gpui::MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.input_mouse_down(
                            TextField::EditorSearch,
                            f32::from(event.position.x),
                            event.click_count,
                            window,
                            cx,
                        )
                    },
                ),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &open_gpui::MouseUpEvent, _window, cx| {
                    this.input_mouse_up(
                        TextField::EditorSearch,
                        f32::from(event.position.x),
                        cx,
                    )
                }),
            )
            .on_key_down(cx.listener(|this, event, window, cx| {
                this.on_editor_search_key(event, window, cx)
            }))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .w(px(220.0))
            .bg(theme::card_inner())
            .border_1()
            .border_color(if focused {
                theme::accent()
            } else {
                theme::border()
            })
            .rounded(px(8.0))
            .cursor_text()
            .child(icon("lucide-search", 14.0, theme::muted()))
            .child(text_row)
            .child(clear)
    }

    // -- ações ----------------------------------------------------------

    pub(crate) fn open_editor(&self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let draft = {
            let s = self.shared.lock().unwrap();
            s.sounds.iter().find(|snd| snd.name == name).map(|snd| {
                (
                    sound_label(snd),
                    if snd.emoji.is_empty() {
                        crate::backend::sounds::DEFAULT_EMOJI.to_string()
                    } else {
                        snd.emoji.clone()
                    },
                )
            })
        };
        let Some((display, emoji)) = draft else {
            return;
        };
        let group = emoji::data()
            .groups
            .first()
            .map(|g| g.label.clone())
            .unwrap_or_default();
        {
            let mut s = self.shared.lock().unwrap();
            s.editor_open = true;
            s.editor_closing = false;
            s.editor_name = name;
            s.editor_display = display;
            s.editor_emoji = emoji;
            s.editor_group = group;
            s.editor_emoji_query.clear();
            super::main_window::focus_field(&mut s, TextField::EditorName);
            s.editor_name_selected = false;
            s.editor_search_selected = false;
            s.bump();
        }
        window.focus(&self.editor_focus, cx);
        cx.notify();
    }

    pub(crate) fn close_editor(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.editor_open && !s.editor_closing {
            // Mantém montado: o fade reverso toca, o tick desmonta.
            s.editor_closing = true;
            s.editor_anim_start = super::format::now_ms();
            s.bump();
            cx.notify();
        }
    }

    pub(crate) fn save_editor(&self, cx: &mut Context<Self>) {
        let (name, display, emoji) = {
            let s = self.shared.lock().unwrap();
            (
                s.editor_name.clone(),
                s.editor_display.clone(),
                s.editor_emoji.clone(),
            )
        };
        if name.is_empty() {
            self.close_editor(cx);
            return;
        }
        backend::sounds::save_metadata(&name, &display, &emoji);
        log::info!("metadata salva: {name}");
        {
            let mut s = self.shared.lock().unwrap();
            s.sounds = backend::list_sounds();
            // Sai com fade em vez de sumir de imediato.
            if s.editor_open && !s.editor_closing {
                s.editor_closing = true;
                s.editor_anim_start = super::format::now_ms();
            }
            s.bump();
        }
        cx.notify();
    }

    pub(crate) fn set_editor_group(&self, group: String, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.editor_group = group;
        s.bump();
        cx.notify();
    }

    pub(crate) fn pick_editor_emoji(&self, emoji: String, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.editor_emoji = emoji;
        s.bump();
        cx.notify();
    }

    pub(crate) fn clear_editor_search(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.editor_emoji_query.clear();
        s.editor_search_selected = false;
        s.bump();
        cx.notify();
    }

    pub(crate) fn focus_editor_name(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.editor_focus, cx);
        let mut s = self.shared.lock().unwrap();
        super::main_window::focus_field(&mut s, TextField::EditorName);
        // Sem mexer na seleção (ver focus_search).
        s.bump();
        cx.notify();
    }

    pub(crate) fn focus_editor_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.editor_search_focus, cx);
        let mut s = self.shared.lock().unwrap();
        super::main_window::focus_field(&mut s, TextField::EditorSearch);
        s.bump();
        cx.notify();
    }

    /// Tira o foco dos inputs do dialog (clique fora deles).
    pub(crate) fn blur_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.editor_name_focused || s.editor_search_focused {
            s.editor_name_focused = false;
            s.editor_search_focused = false;
            s.bump();
            window.blur();
            cx.notify();
        }
    }

    pub(crate) fn on_editor_name_key(
        &self,
        event: &open_gpui::KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ks = &event.keystroke;
        if (ks.modifiers.control || ks.modifiers.platform)
            && !ks.modifiers.alt
            && ks.key.eq_ignore_ascii_case("a")
        {
            let mut s = self.shared.lock().unwrap();
            if !s.editor_display.is_empty() {
                s.editor_name_selected = true;
                s.bump();
                cx.notify();
            }
            return;
        }
        let mut s = self.shared.lock().unwrap();
        match ks.key.as_str() {
            "backspace" => {
                if s.editor_name_selected {
                    s.editor_display.clear();
                    s.editor_name_selected = false;
                } else {
                    s.editor_display.pop();
                }
            }
            "escape" => {
                if s.editor_name_selected {
                    s.editor_name_selected = false;
                } else {
                    s.bump();
                    drop(s);
                    self.close_editor(cx);
                    return;
                }
            }
            "enter" => {
                s.editor_name_selected = false;
                s.bump();
                drop(s);
                self.save_editor(cx);
                return;
            }
            _ => {
                if !ks.modifiers.control && !ks.modifiers.alt && !ks.modifiers.platform {
                    if let Some(ch) = ks.key_char.clone() {
                        if ch.chars().count() == 1 {
                            if s.editor_name_selected {
                                s.editor_display = ch;
                                s.editor_name_selected = false;
                            } else {
                                s.editor_display.push_str(&ch);
                            }
                        } else if s.editor_name_selected {
                            s.editor_name_selected = false;
                        }
                    } else if s.editor_name_selected {
                        s.editor_name_selected = false;
                    }
                } else {
                    return;
                }
            }
        }
        s.bump();
        cx.notify();
    }

    pub(crate) fn on_editor_search_key(
        &self,
        event: &open_gpui::KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ks = &event.keystroke;
        if (ks.modifiers.control || ks.modifiers.platform)
            && !ks.modifiers.alt
            && ks.key.eq_ignore_ascii_case("a")
        {
            let mut s = self.shared.lock().unwrap();
            if !s.editor_emoji_query.is_empty() {
                s.editor_search_selected = true;
                s.bump();
                cx.notify();
            }
            return;
        }
        let mut s = self.shared.lock().unwrap();
        match ks.key.as_str() {
            "backspace" => {
                if s.editor_search_selected {
                    s.editor_emoji_query.clear();
                    s.editor_search_selected = false;
                } else {
                    s.editor_emoji_query.pop();
                }
            }
            "escape" => {
                if s.editor_search_selected {
                    s.editor_search_selected = false;
                } else if !s.editor_emoji_query.is_empty() {
                    s.editor_emoji_query.clear();
                } else {
                    s.bump();
                    drop(s);
                    self.close_editor(cx);
                    return;
                }
            }
            "enter" => {
                // Confirma o primeiro resultado, se houver.
                let lang = s.lang.clone();
                let query = s.editor_emoji_query.clone();
                drop(s);
                if let Some((emoji, _)) = emoji::search_emoji(&lang, &query).into_iter().next() {
                    self.pick_editor_emoji(emoji, cx);
                }
                return;
            }
            _ => {
                if !ks.modifiers.control && !ks.modifiers.alt && !ks.modifiers.platform {
                    if let Some(ch) = ks.key_char.clone() {
                        if ch.chars().count() == 1 {
                            if s.editor_search_selected {
                                s.editor_emoji_query = ch;
                                s.editor_search_selected = false;
                            } else {
                                s.editor_emoji_query.push_str(&ch);
                            }
                        } else if s.editor_search_selected {
                            s.editor_search_selected = false;
                        }
                    } else if s.editor_search_selected {
                        s.editor_search_selected = false;
                    }
                } else {
                    return;
                }
            }
        }
        s.bump();
        cx.notify();
    }
}

fn editor_tabs(active: &str, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div()
        .id("editor-tabs")
        .flex()
        .flex_row()
        .gap_1()
        .p_1()
        .overflow_x_scroll()
        .bg(theme::card_inner())
        .border_1()
        .border_color(theme::border())
        .rounded(px(8.0))
        .children(emoji::data().groups.iter().map(|group| {
            let label = group.label.clone();
            let is_active = label == active;
            div()
                .id(format!("editor-tab-{label}"))
                .on_click(cx.listener(move |this, _e, _w, cx| {
                    cx.stop_propagation();
                    this.set_editor_group(label.clone(), cx)
                }))
                .flex_shrink_0()
                .px_3()
                .py_2()
                .rounded(px(6.0))
                .cursor_pointer()
                .bg(if is_active {
                    theme::card_hover()
                } else {
                    open_gpui::rgba(0x00000000)
                })
                .hover(|s| s.bg(theme::card_hover()))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if is_active {
                    theme::text()
                } else {
                    theme::muted()
                })
                .child(group.label.clone())
        }))
}

fn editor_grid(
    lang: &str,
    query: &str,
    group: &str,
    picked: &str,
    shared: &std::sync::Arc<std::sync::Mutex<crate::core::state::Shared>>,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let body: open_gpui::AnyElement = if !query.trim().is_empty() {
        let results = emoji::search_emoji(lang, query);
        if results.is_empty() {
            div()
                .py_8()
                .flex()
                .justify_center()
                .text_sm()
                .text_color(theme::muted())
                .child(t(lang, "editor.no_results"))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .children(results.iter().enumerate().map(|(i, (emoji, _))| {
                    emoji_button(
                        format!("editor-sr-{i}"),
                        emoji,
                        picked == emoji,
                        shared,
                        cx,
                    )
                }))
                .into_any_element()
        }
    } else {
        let subgroups: Vec<(String, Vec<(String, String)>)> = emoji::data()
            .groups
            .iter()
            .find(|g| g.label == group)
            .map(|g| {
                g.subgroups
                    .iter()
                    .map(|s| {
                        (
                            s.label.clone(),
                            s.emojis
                                .iter()
                                .map(|o| (o.emoji.clone(), o.name.clone()))
                                .collect(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .children(subgroups.iter().enumerate().map(|(si, (label, emojis))| {
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::muted())
                            .child(label.to_uppercase().replace('-', " ")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap_2()
                            .children(emojis.iter().enumerate().map(|(ei, (emoji, _))| {
                                emoji_button(
                                    format!("editor-em-{si}-{ei}"),
                                    emoji,
                                    picked == emoji,
                                    shared,
                                    cx,
                                )
                            })),
                    )
                    .into_any_element()
            }))
            .into_any_element()
    };
    div()
        .id("editor-grid")
        .flex_1()
        .overflow_y_scroll()
        .p_2()
        .bg(theme::card_inner())
        .border_1()
        .border_color(theme::border())
        .rounded(px(8.0))
        .child(body)
}

fn emoji_button(
    id: String,
    emoji: &str,
    selected: bool,
    shared: &std::sync::Arc<std::sync::Mutex<crate::core::state::Shared>>,
    cx: &mut Context<MainWindow>,
) -> open_gpui::AnyElement {
    let emoji = emoji.to_string();
    let pick = emoji.clone();
    // Raster próprio (swash) em vez de texto: imune a fallback de fontes.
    // Enquanto o worker não entrega, caixa vazia (botão segue clicável).
    let glyph: open_gpui::AnyElement = match super::emoji::image(&emoji, shared) {
        Some(image) => open_gpui::img(image)
            .w(px(28.0))
            .h(px(28.0))
            .into_any_element(),
        None => div().size(px(28.0)).into_any_element(),
    };
    div()
        .id(id)
        .on_click(cx.listener(move |this, _e, _w, cx| {
            cx.stop_propagation();
            this.pick_editor_emoji(pick.clone(), cx)
        }))
        .size(px(40.0))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .rounded(px(6.0))
        .cursor_pointer()
        .border_1()
        .border_color(if selected {
            theme::accent()
        } else {
            open_gpui::rgba(0x00000000)
        })
        .bg(if selected {
            theme::sel_bg()
        } else {
            open_gpui::rgba(0x00000000)
        })
        .hover(|s| s.bg(theme::card_hover()).border_color(theme::accent()))
        .child(glyph)
        .into_any_element()
}
