use std::time::Duration;

use open_gpui::{
    div, px, Animation, AnimationExt, Context, FocusHandle, FontWeight, IntoElement,
    ease_out_quint, prelude::*,
};

use super::icons::icon;
use super::main_window::MainWindow;
use super::theme;
use crate::core::i18n::t;
use super::ui::icon_action_btn;

/// Drawer lateral de settings (Image 2). Backdrop fecha ao clicar fora.
impl MainWindow {
    pub(crate) fn settings_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (open, closing, mic_pass, hear, run_in_background, mic_source, mic_sources, mic_open, shortcut, shortcut_error, theme_pref, lang, lang_open, rebinding) = {
            let s = self.shared.lock().unwrap();
            (
                s.settings_open,
                s.settings_closing,
                s.mic_pass,
                s.hear_clips,
                s.run_in_background,
                s.mic_source.clone(),
                s.mic_sources.clone(),
                s.mic_open,
                s.shortcut.clone(),
                s.shortcut_error.clone(),
                s.theme.clone(),
                s.lang.clone(),
                s.lang_open,
                s.shortcut_rebinding,
            )
        };
        if !open {
            return div().into_any_element();
        }
        let panel = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::panel())
            .border_l_1()
            .border_color(theme::border())
            .id("settings-panel")
            .on_click(cx.listener(|_this, _e, _w, cx| {
                cx.stop_propagation();
            }))
            .child(
            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px_5()
                                .py_4()
                                .border_b_1()
                                .border_color(theme::border())
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(t(&lang, "settings.title")),
                                )
                                .child(
                                    icon_action_btn("lucide-x", theme::muted(), 16.0)
                                        .id("settings-close")
                                        .on_click(cx.listener(|this, _e, _w, cx| this.close_settings(cx)),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .id("settings-scroll")
                                .flex_1()
                                .overflow_y_scroll()
                                .flex()
                                .flex_col()
                                .gap_6()
                                .px_5()
                                .py_5()
                                .child(section_label(&t(&lang, "settings.mic_group")))
                                .child(self.switch_row(
                                    mic_pass,
                                    &t(&lang, "settings.mic_label"),
                                    "mic-pass",
                                    SwitchTarget::MicPass,
                                    cx,
                                ))
                                .child(mic_select(&lang, &mic_sources, &mic_source, mic_open, cx))
                                .child(section_label(&t(&lang, "settings.mon_group")))
                                .child(self.switch_row(
                                    hear,
                                    &t(&lang, "settings.hear_label"),
                                    "hear-clips",
                                    SwitchTarget::HearClips,
                                    cx,
                                ))
                                .child(section_label(&t(&lang, "settings.theme_group")))
                                .child(theme_segment(&lang, &theme_pref, cx))
                                .child(section_label(&t(&lang, "settings.lang_group")))
                                .child(language_select(&lang, lang_open, cx))
                                .child(section_label(&t(&lang, "settings.shortcut_group")))
                                .child(shortcut_box(&lang, &shortcut, rebinding, &self.shortcut_focus, cx))
                                .child(shortcut_error_note(shortcut_error))
                                .child(section_label(&t(&lang, "settings.tray_group")))
                                .child(self.switch_row(
                                    run_in_background,
                                    &t(&lang, "settings.tray_label"),
                                    "tray-bg",
                                    SwitchTarget::RunInBackground,
                                    cx,
                                ))
                                .child(section_label(&t(&lang, "settings.discord_group")))
                                .child(discord_hint(&lang))
                                .child(about_button(&lang, cx))
                    )
        ;
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
                    .id("settings-backdrop")
                    .on_click(cx.listener(|this, _e, _w, cx| {
                            cx.stop_propagation();
                            this.close_settings(cx)
                        }),
                    )
                    .with_animation(
                        if closing {
                            "settings-backdrop-out"
                        } else {
                            "settings-backdrop"
                        },
                        Animation::new(Duration::from_millis(180)),
                        move |el, delta| {
                            let d = if closing { 1.0 - delta } else { delta };
                            let a = (0x99 as f32 * d) as u32;
                            el.bg(open_gpui::rgba(a))
                        },
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .w(px(360.0))
                    .child(panel)
                    .with_animation(
                        if closing {
                            "settings-drawer-out"
                        } else {
                            "settings-drawer"
                        },
                        Animation::new(Duration::from_millis(220))
                            .with_easing(ease_out_quint()),
                        move |el, delta| {
                            let d = if closing { delta } else { 1.0 - delta };
                            el.right(px(-360.0 * d))
                        },
                    ),
            )
            .into_any_element()
    }

    fn switch_row(
        &self,
        on: bool,
        label: &str,
        id: &str,
        target: SwitchTarget,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .child(switch(on, id, target, cx))
            // `min_w(0)`: sem isso o item flex não encolhe e o texto
            // (ex. "Pass my microphone through…") estoura o drawer
            // em vez de quebrar linha.
            .child(div().flex_1().min_w(px(0.0)).text_sm().child(label.to_string()))
    }
}

#[derive(Clone, Copy)]
enum SwitchTarget {
    MicPass,
    HearClips,
    RunInBackground,
}

/// Bolinha animada via `with_animation`: o id inclui o estado, então
/// cada toggle remonta e toca a animação a 60fps — só no switch clicado.
fn switch(
    on: bool,
    id: &str,
    target: SwitchTarget,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    const OFF_X: f32 = 3.0;
    const ON_X: f32 = 25.0;
    let (from, to) = if on { (OFF_X, ON_X) } else { (ON_X, OFF_X) };
    div()
        .id(id.to_string())
        .on_click(cx.listener(move |this, _e, _w, cx| {
                cx.stop_propagation();
                match target {
                    SwitchTarget::MicPass => this.set_mic_pass(!on, cx),
                    SwitchTarget::HearClips => this.set_hear_clips(!on, cx),
                    SwitchTarget::RunInBackground => this.set_run_in_background(!on, cx),
                }
            }),
        )
        .w(px(44.0))
        .h(px(24.0))
        .flex_shrink_0()
        .relative()
        .rounded(px(999.0))
        .cursor_pointer()
        .bg(if on { theme::accent() } else { theme::card() })
        .border_1()
        .border_color(if on { theme::accent() } else { theme::border() })
        .child(
            div()
                .absolute()
                .top(px(3.0))
                .left(px(from))
                .size(px(16.0))
                .bg(if on {
                    open_gpui::rgb(0xf2f8ff)
                } else {
                    theme::muted()
                })
                .rounded(px(999.0))
                .with_animation(
                    format!("{id}-knob-{}", if on { "on" } else { "off" }),
                    Animation::new(Duration::from_millis(180))
                        .with_easing(ease_out_quint()),
                    move |el, delta| el.left(px(from + (to - from) * delta)),
                ),
        )
}

fn section_label(text: &str) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::muted())
        .child(text.to_string())
}

fn mic_select(
    lang: &str,
    sources: &[(String, String)],
    current: &str,
    open: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let current_desc = sources
        .iter()
        .find(|(n, _)| n == current)
        .map(|(_, d)| d.clone())
        .unwrap_or_else(|| {
            if current.is_empty() {
                t(lang, "settings.mic_none")
            } else {
                current.to_string()
            }
        });
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
                sources
                    .iter()
                    .map(|(name, desc)| {
                        let name = name.clone();
                        let selected = name == current;
                        div()
                            .id(format!("mic-{name}"))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                    cx.stop_propagation();
                                    this.select_mic_source(name.clone(), cx)
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
                            .hover(|s| s.bg(theme::card_hover()))
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
                                    .child(desc.clone()),
                            )
                            .child(if selected {
                                icon("lucide-check", 14.0, theme::accent()).into_any_element()
                            } else {
                                div().into_any_element()
                            })
                    })
                    .collect::<Vec<_>>(),
            );
        // `deferred`: pinta por cima das seções seguintes, sem empurrar o layout.
        // Fade-in de 120ms na abertura.
        open_gpui::deferred(super::ui::fade_in(list, "mic-options".to_string(), 120))
            .into_any_element()
    } else {
        div().into_any_element()
    };
    div()
        .relative()
        .flex()
        .flex_col()
        .child(
            div()
                .id("mic-select")
                .on_click(cx.listener(|this, _e, _w, cx| this.toggle_mic_open(cx)),
                )
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .px_3()
                .py_2()
                .bg(theme::card())
                .border_1()
                .border_color(theme::border())
                .hover(|s| s.border_color(theme::dashed()))
                .rounded(px(8.0))
                .cursor_pointer()
                .child(div().flex_1().text_sm().truncate().child(current_desc))
                .child(icon(
                    if open { "lucide-chevron-up" } else { "lucide-chevron-down" },
                    14.0,
                    theme::muted(),
                )),
        )
        .child(options)
}

fn theme_segment(lang: &str, current: &str, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .child(theme_card(
            "lucide-sun",
            &t(lang, "settings.theme_light"),
            "light",
            current,
            cx,
        ))
        .child(theme_card(
            "lucide-moon",
            &t(lang, "settings.theme_dark"),
            "dark",
            current,
            cx,
        ))
        .child(theme_card(
            "lucide-sun-moon",
            &t(lang, "settings.theme_system"),
            "system",
            current,
            cx,
        ))
}

fn theme_card(
    asset: &str,
    label: &str,
    key: &'static str,
    current: &str,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let active = current == key;
    let label = label.to_string();
    let asset = asset.to_string();
    div()
        .id(format!("theme-{label}"))
        .on_click(cx.listener(move |this, _e, _w, cx| {
                cx.stop_propagation();
                this.set_theme(key, cx);
            }),
        )
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .py_3()
        .bg(theme::card())
        .border_1()
        .border_color(if active { theme::accent() } else { theme::border() })
        .rounded(px(10.0))
        .cursor_pointer()
        .hover(|s| s.bg(theme::card_hover()))
        .child(icon(
            &asset,
            18.0,
            if active { theme::accent() } else { theme::muted() },
        ))
        .child(
            div()
                .text_sm()
                .text_color(if active { theme::accent() } else { theme::muted() })
                .child(label),
        )
}

const LANGS: &[&str] = &["system", "en", "pt-BR"];

fn lang_label(lang: &str, key: &str) -> String {
    match key {
        "system" => t(lang, "settings.lang_system"),
        "en" => t(lang, "settings.lang_en"),
        _ => t(lang, "settings.lang_pt"),
    }
}

/// Seletor de idioma funcional (System/English/Português).
fn language_select(lang: &str, open: bool, cx: &mut Context<MainWindow>) -> impl IntoElement {
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
                LANGS
                    .iter()
                    .map(|name| {
                        let key = name.to_string();
                        let selected = key == crate::core::i18n::resolve(lang);
                        let selected_label = lang_label(lang, &key);
                        div()
                            .id(format!("lang-{key}"))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                    cx.stop_propagation();
                                    this.set_language(&key, cx)
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
                            .hover(|s| s.bg(theme::card_hover()))
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
                                    .child(selected_label),
                            )
                            .child(if selected {
                                icon("lucide-check", 14.0, theme::accent()).into_any_element()
                            } else {
                                div().into_any_element()
                            })
                    })
                    .collect::<Vec<_>>(),
            );
        open_gpui::deferred(super::ui::fade_in(list, "lang-options".to_string(), 120))
            .into_any_element()
    } else {
        div().into_any_element()
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .child(icon("lucide-globe", 16.0, theme::muted()))
        .child(
            div()
                .relative()
                .flex()
                .flex_col()
                .child(
                    div()
                        .id("lang-select")
                        .on_click(cx.listener(|this, _e, _w, cx| this.toggle_lang_open(cx)),
                        )
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .px_3()
                        .py_2()
                        .w(px(180.0))
                        .bg(theme::card())
                        .border_1()
                        .border_color(theme::border())
                        .hover(|s| s.border_color(theme::dashed()))
                        .rounded(px(8.0))
                        .cursor_pointer()
                        .child(
                            div()
                                .flex_1()
                                .text_sm()
                                .truncate()
                                .child(lang_label(lang, crate::core::i18n::resolve(lang))),
                        )
                        .child(icon(
                            if open { "lucide-chevron-up" } else { "lucide-chevron-down" },
                            14.0,
                            theme::muted(),
                        )),
                )
                .child(options),
        )
}

fn shortcut_box(
    lang: &str,
    shortcut: &str,
    rebinding: bool,
    focus: &FocusHandle,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    div()
        .id("shortcut-box")
        .track_focus(focus)
        .on_key_down(cx.listener(|this, event, window, cx| {
            this.on_shortcut_record(event, window, cx)
        }))
        .on_click(cx.listener(|this, _e, window, cx| this.rebind_shortcut(window, cx)),
        )
        .flex()
        .items_center()
        .justify_center()
        .px_3()
        .py_2()
        .bg(theme::card())
        .border_1()
        .border_color(if rebinding {
            theme::accent()
        } else {
            theme::border()
        })
        .hover(|s| s.border_color(theme::accent()))
        .rounded(px(8.0))
        .cursor_pointer()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(if rebinding {
            theme::accent()
        } else {
            theme::text()
        })
        .child(if rebinding {
            t(lang, "settings.shortcut_rec")
        } else {
            shortcut.to_string()
        })
}

/// Erro do atalho global, logo abaixo da caixa (contexto imediato).
fn shortcut_error_note(err: Option<String>) -> impl IntoElement {
    match err {
        Some(msg) => div().text_sm().text_color(theme::danger()).child(msg),
        None => div(),
    }
}

fn discord_hint(lang: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_1()
        .text_sm()
        .text_color(theme::muted())
        .child(t(lang, "settings.discord_intro"))
        .child(
            div()
                .px_2()
                .bg(theme::card())
                .border_1()
                .border_color(theme::border())
                .rounded(px(6.0))
                .text_xs()
                .text_color(theme::text())
                .child(t(lang, "settings.discord_input")),
        )
        .child(t(lang, "settings.discord_to"))
        .child(
            div()
                .px_2()
                .bg(theme::card())
                .border_1()
                .border_color(theme::border())
                .rounded(px(6.0))
                .text_xs()
                .text_color(theme::text())
                .child("Klipp-Mic"),
        )
}

/// Botão "About Klipp" no fim do drawer (Image 1).
fn about_button(lang: &str, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div()
        .id("about-open")
        .on_click(cx.listener(|this, _e, _w, cx| this.open_about(cx)),
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
        .child(icon("lucide-circle-help", 16.0, theme::text()))
        .child(t(lang, "settings.about"))
}
