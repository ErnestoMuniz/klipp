use std::time::Duration;

use gpui_animation::animation::TransitionExt;
use gpui_animation::transition::general::Linear;
use open_gpui::{Context, Div, IntoElement, MouseButton, Rgba, Stateful, div, prelude::*, px};

use super::icons::icon;
use super::main_window::{MainWindow, TextField};
use super::theme;
use super::ui::{caret, fade_in};
use crate::core::i18n::t;

/// Botão-ícone da toolbar (padding, cantos, cursor). Sem hover estático:
/// o chamador aplica `with_hover_bg` para o fade de background.
/// A base é transparente: o fade interpola o alfa, sem acoplar à cor do fundo.
fn toolbar_btn(asset: &str, color: Rgba, size_px: f32) -> Div {
    div()
        .p_2()
        .rounded(px(6.0))
        .cursor_pointer()
        .bg(open_gpui::rgba(0x00000000))
        .child(icon(asset, size_px, color))
}

/// Fade de background transparente -> `card_hover` no hover (100ms linear).
fn with_hover_bg(el: Stateful<Div>, tid: impl Into<open_gpui::ElementId>) -> impl IntoElement {
    el.with_transition(tid).transition_on_hover(
        Duration::from_millis(100),
        Linear,
        |hovered, s| {
            if *hovered {
                s.bg(theme::card_hover())
            } else {
                s.bg(open_gpui::rgba(0x00000000))
            }
        },
    )
}

/// Fade de borda `border` -> `dashed` no hover (100ms linear).
fn with_hover_border(el: Stateful<Div>, tid: impl Into<open_gpui::ElementId>) -> impl IntoElement {
    el.with_transition(tid).transition_on_hover(
        Duration::from_millis(100),
        Linear,
        |hovered, s| {
            if *hovered {
                s.border_color(theme::dashed())
            } else {
                s.border_color(theme::border())
            }
        },
    )
}

impl MainWindow {
    pub(crate) fn toolbar(
        &self,
        total: usize,
        visible: usize,
        query: &str,
        search_focused: bool,
        hints_on: bool,
        only_fav: bool,
        compact: bool,
        settings_open: bool,
        browse_open: bool,
        sort: &str,
        sort_open: bool,
        lang: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let count_label = if query.is_empty() {
            format!("{total}")
        } else {
            format!("{visible}/{total}")
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .p_2()
                            .bg(theme::card())
                            .border_1()
                            .border_color(theme::border())
                            .rounded(px(6.0))
                            .text_color(theme::accent())
                            .child(icon("lucide-music", 13.0, theme::accent())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(open_gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::muted())
                            .child(t(lang, "toolbar.library").to_uppercase()),
                    )
                    .child(
                        div()
                            .px_2()
                            .rounded(px(999.0))
                            .border_1()
                            .border_color(theme::accent())
                            .text_xs()
                            .text_color(theme::accent())
                            .child(count_label),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(self.search_box(query, search_focused, lang, cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .p_1()
                            .bg(theme::panel())
                            .border_1()
                            .border_color(theme::border())
                            .rounded(px(8.0))
                            .child(
                                with_hover_bg(
                                    toolbar_btn(
                                        "lucide-layout-grid",
                                        if compact { theme::muted() } else { theme::accent() },
                                        14.0,
                                    )
                                    .id("density-comfort")
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                            this.set_density("comfort", cx)
                                        }),
                                    ),
                                    "tb-density-comfort-bg",
                                ),
                            )
                            .child(
                                with_hover_bg(
                                    toolbar_btn(
                                        "lucide-rows-3",
                                        if compact { theme::accent() } else { theme::muted() },
                                        14.0,
                                    )
                                    .id("density-compact")
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                            this.set_density("compact", cx)
                                        }),
                                    ),
                                    "tb-density-compact-bg",
                                ),
                            ),
                    )
                    // Ordenação
                    .child(sort_select(sort, sort_open, lang, cx))
                    .child(
                        with_hover_bg(
                            toolbar_btn(
                                if only_fav { "lucide-star-filled" } else { "lucide-star" },
                                if only_fav { theme::accent() } else { theme::muted() },
                                18.0,
                            )
                            .id("fav-filter")
                            .on_click(cx.listener(|this, _e, _w, cx| this.toggle_only_favorites(cx)),
                            ),
                            "tb-fav-filter-bg",
                        ),
                    )
                    .child(with_hover_bg(
                        toolbar_btn(
                            "lucide-cloud",
                            if browse_open { theme::accent() } else { theme::muted() },
                            18.0,
                        )
                        .id("tb-cloud")
                        .on_click(cx.listener(|this, _e, window, cx| {
                            this.toggle_browse(window, cx)
                        })),
                        "tb-cloud-bg",
                    ))
                    .child(
                        with_hover_bg(
                            toolbar_btn(
                                "lucide-lightbulb",
                                if hints_on { theme::accent() } else { theme::muted() },
                                18.0,
                            )
                            .id("hints-toggle")
                            .on_click(cx.listener(|this, _e, _w, cx| this.toggle_hints(cx)),
                            ),
                            "tb-hints-toggle-bg",
                        ),
                    )
                    .child(
                        with_hover_bg(
                            toolbar_btn(
                                "lucide-settings",
                                if settings_open {
                                    theme::accent()
                                } else {
                                    theme::muted()
                                },
                                20.0,
                            )
                            .id("settings-toggle")
                            .on_click(cx.listener(|this, _e, _w, cx| this.toggle_settings(cx)),
                            ),
                            "tb-settings-toggle-bg",
                        ),
                    ),
            )
    }

    fn search_box(
        &self,
        query: &str,
        focused: bool,
        lang: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let border = if focused { theme::accent() } else { theme::border() };
        // Ctrl+A: destaca o texto todo com o fundo de seleção.
        let selected = !query.is_empty() && self.shared.lock().unwrap().search_selected;
        let text: open_gpui::AnyElement = if query.is_empty() {
            div()
                .text_sm()
                .truncate()
                .text_color(theme::muted())
                .child(t(lang, "toolbar.search_ph"))
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
            with_hover_bg(
                div()
                    .id("search-clear")
                    .on_click(cx.listener(|this, _e, _w, cx| this.clear_search(cx)),
                    )
                    .p_1()
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .bg(open_gpui::rgba(0x00000000))
                    .child(icon("lucide-x", 11.0, theme::muted())),
                "tb-search-clear-bg",
            )
            .into_any_element()
        };
        // Caret: início do campo vazio, logo após o texto caso contrário.
        let text_row: open_gpui::AnyElement = if query.is_empty() {
            div()
                .flex_1()
                .flex()
                .flex_row()
                .items_center()
                .child(caret(focused, theme::panel(), 16.0))
                .child(text)
                .into_any_element()
        } else {
            div()
                .flex_1()
                .flex()
                .flex_row()
                .items_center()
                .child(text)
                .child(caret(focused, theme::panel(), 16.0))
                .into_any_element()
        };
        div()
            .id("search-box")
            .track_focus(&self.search_focus)
            .on_click(cx.listener(|this, _e, window, cx| {
                    cx.stop_propagation();
                    this.focus_search(window, cx)
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(
                    |this, event: &open_gpui::MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.input_mouse_down(
                            TextField::Search,
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
                    this.input_mouse_up(TextField::Search, f32::from(event.position.x), cx)
                }),
            )
            .on_key_down(cx.listener(|this, event, window, cx| {
                this.on_search_key(event, window, cx)
            }))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .w(px(260.0))
            .bg(theme::panel())
            .border_1()
            .border_color(border)
            .rounded(px(8.0))
            .cursor_text()
            .child(icon("lucide-search", 17.0, theme::muted()))
            .child(text_row)
            .child(clear)
            .with_transition("tb-search-box")
            .transition_on_hover(
                Duration::from_millis(100),
                Linear,
                move |hovered, s| {
                    if *hovered {
                        s.border_color(theme::dashed())
                    } else if focused {
                        s.border_color(theme::accent())
                    } else {
                        s.border_color(theme::border())
                    }
                },
            )
            .transition_when_else(
                focused,
                Duration::from_millis(100),
                Linear,
                |s| s.border_color(theme::accent()),
                |s| s.border_color(theme::border()),
            )
    }
}

const SORTS: &[&str] = &["name-asc", "name-desc", "recent"];

fn sort_label(lang: &str, sort: &str) -> String {
    match sort {
        "name-desc" => t(lang, "toolbar.sort_desc"),
        "recent" => t(lang, "toolbar.sort_recent"),
        _ => t(lang, "toolbar.sort_asc"),
    }
}

/// Seletor de ordenação: trigger + painel absoluto com fade.
fn sort_select(sort: &str, open: bool, lang: &str, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let options: open_gpui::AnyElement = if open {
        let list = div()
            .absolute()
            .top(px(44.0))
            .left(px(0.0))
            .right(px(0.0))
            .flex()
            .flex_col()
            .gap_1()
            .p_1()
            .bg(theme::panel())
            .border_1()
            .border_color(theme::border())
            .rounded(px(8.0))
            .children(
                SORTS
                    .iter()
                    .map(|name| {
                        let key = name.to_string();
                        let selected = key == sort;
                        let label = sort_label(lang, &key);
                        let tid = format!("tb-sort-opt-{key}");
                        div()
                            .id(format!("sort-{key}"))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                    cx.stop_propagation();
                                    this.set_sort(&key, cx)
                                }),
                            )
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .px_3()
                            .py_2()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if selected {
                                theme::sel_bg()
                            } else {
                                theme::panel()
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .truncate()
                                    .text_color(if selected {
                                        theme::accent()
                                    } else {
                                        theme::text()
                                    })
                                    .child(label.to_string()),
                            )
                            .child(if selected {
                                icon("lucide-check", 14.0, theme::accent()).into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .with_transition(tid)
                            .transition_on_hover(
                                Duration::from_millis(100),
                                Linear,
                                move |hovered, s| {
                                    if *hovered {
                                        s.bg(theme::card_hover())
                                    } else if selected {
                                        s.bg(theme::sel_bg())
                                    } else {
                                        s.bg(theme::panel())
                                    }
                                },
                            )
                    })
                    .collect::<Vec<_>>(),
            );
        open_gpui::deferred(fade_in(list, "sort-options".to_string(), 120)).into_any_element()
    } else {
        div().into_any_element()
    };
    div()
        .relative()
        .flex()
        .flex_col()
        .child(
            with_hover_border(
                div()
                    .id("sort-select")
                    .on_click(cx.listener(|this, _e, _w, cx| this.toggle_sort_open(cx)),
                    )
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .bg(theme::panel())
                    .border_1()
                    .border_color(theme::border())
                    .rounded(px(8.0))
                    .cursor_pointer()
                    .child(div().text_sm().child(sort_label(lang, sort)))
                    .child(icon(
                        if open { "lucide-chevron-up" } else { "lucide-chevron-down" },
                        14.0,
                        theme::muted(),
                    )),
                "tb-sort-select",
            ),
        )
        .child(options)
}
