use std::time::Duration;

use open_gpui::{
    Context, FontWeight, IntoElement, MouseButton, Window, div, ease_out_quint, prelude::*, px,
    Animation, AnimationExt,
};

use super::icons::icon;
use super::main_window::{MainWindow, TextField};
use super::theme;
use super::ui::{caret, icon_action_btn};
use crate::backend;
use crate::core::i18n::{t, t_fmt};
use crate::core::state::{OnlineSound, Sound};

/// Drawer lateral "Browse sounds online" (MyInstants).
/// Mesmo padrão do drawer de settings: backdrop fecha ao clicar fora,
/// animação de entrada/saída, painel de 400px à direita.
impl MainWindow {
    pub(crate) fn browse_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (open, closing, lang, query, searched, focused, results, searching, error, downloading, previewing, preview_loading, playing, sounds) = {
            let s = self.shared.lock().unwrap();
            (
                s.browse_open,
                s.browse_closing,
                s.lang.clone(),
                s.browse_query.clone(),
                s.browse_searched.clone(),
                s.browse_focused,
                s.browse_results.clone(),
                s.browse_searching,
                s.browse_error.clone(),
                s.browse_downloading.clone(),
                s.browse_previewing.clone(),
                s.browse_preview_loading,
                s.playing.clone(),
                s.sounds.clone(),
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
            .id("browse-panel")
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
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(icon("lucide-cloud", 18.0, theme::text()))
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t(&lang, "browse.title")),
                            ),
                    )
                    .child(
                        icon_action_btn("lucide-x", theme::muted(), 16.0)
                            .id("browse-close")
                            .on_click(cx.listener(|this, _e, _w, cx| this.close_browse(cx))),
                    ),
            )
            .child(self.browse_search_box(&lang, &query, focused, cx))
            .child(
                div()
                    .px_5()
                    .pt_2()
                    .text_xs()
                    .text_color(theme::muted())
                    .child(t(&lang, "browse.source")),
            )
            .child(browse_results(
                &lang,
                &query,
                &searched,
                &results,
                &sounds,
                searching,
                error.as_deref(),
                downloading.as_deref(),
                previewing.as_deref(),
                preview_loading,
                playing.as_deref(),
                cx,
            ));
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
                    .id("browse-backdrop")
                    .on_click(cx.listener(|this, _e, _w, cx| {
                        cx.stop_propagation();
                        this.close_browse(cx)
                    }))
                    .with_animation(
                        if closing {
                            "browse-backdrop-out"
                        } else {
                            "browse-backdrop"
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
                    .w(px(400.0))
                    .child(panel)
                    .with_animation(
                        if closing {
                            "browse-drawer-out"
                        } else {
                            "browse-drawer"
                        },
                        Animation::new(Duration::from_millis(220))
                            .with_easing(ease_out_quint()),
                        move |el, delta| {
                            let d = if closing { delta } else { 1.0 - delta };
                            el.right(px(-400.0 * d))
                        },
                    ),
            )
            .into_any_element()
    }

    fn browse_search_box(
        &self,
        lang: &str,
        query: &str,
        focused: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let border = if focused {
            theme::accent()
        } else {
            theme::border()
        };
        // Ctrl+A: destaca o texto todo com o fundo de seleção.
        let selected = !query.is_empty() && self.shared.lock().unwrap().browse_selected;
        let text: open_gpui::AnyElement = if query.is_empty() {
            div()
                .text_sm()
                .truncate()
                .text_color(theme::muted())
                .child(t(lang, "browse.search_ph"))
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
                .id("browse-clear")
                .on_click(cx.listener(|this, _e, _w, cx| this.clear_browse(cx)))
                .into_any_element()
        };
        let text_row: open_gpui::AnyElement = if query.is_empty() {
            div()
                .flex_1()
                .flex()
                .flex_row()
                .items_center()
                .child(caret(focused, theme::card(), 16.0))
                .child(text)
                .into_any_element()
        } else {
            div()
                .flex_1()
                .flex()
                .flex_row()
                .items_center()
                .child(text)
                .child(caret(focused, theme::card(), 16.0))
                .into_any_element()
        };
        div().px_5().pt_4().child(
            div()
                .id("browse-search-box")
                .track_focus(&self.browse_focus)
                .on_click(cx.listener(|this, _e, window, cx| {
                    cx.stop_propagation();
                    this.focus_browse(window, cx)
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(
                        |this, event: &open_gpui::MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.input_mouse_down(
                                TextField::Browse,
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
                        this.input_mouse_up(TextField::Browse, f32::from(event.position.x), cx)
                    }),
                )
                .on_key_down(cx.listener(|this, event, window, cx| {
                    this.on_browse_key(event, window, cx)
                }))
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .bg(theme::card())
                .border_1()
                .border_color(border)
                .rounded(px(8.0))
                .cursor_text()
                .child(icon("lucide-search", 17.0, theme::muted()))
                .child(text_row)
                .child(clear),
        )
    }

    // -- ações ----------------------------------------------------------

    pub(crate) fn toggle_browse(&self, window: &mut Window, cx: &mut Context<Self>) {
        let open = self.shared.lock().unwrap().browse_open;
        if open {
            self.close_browse(cx);
        } else {
            {
                let mut s = self.shared.lock().unwrap();
                s.browse_open = true;
                s.browse_closing = false;
                // Drawer novo = seleção velha não volta.
                s.browse_selected = false;
                s.bump();
            }
            self.focus_browse(window, cx);
            cx.notify();
        }
    }

    pub(crate) fn close_browse(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.browse_open && !s.browse_closing {
            s.browse_closing = true;
            s.browse_anim_start = super::format::now_ms();
            s.bump();
            cx.notify();
        }
    }

    pub(crate) fn focus_browse(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.browse_focus, cx);
        let mut s = self.shared.lock().unwrap();
        super::main_window::focus_field(&mut s, TextField::Browse);
        // Sem mexer na seleção (ver focus_search).
        s.bump();
        cx.notify();
    }

    pub(crate) fn blur_browse(&self, window: &mut Window, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.browse_focused {
            s.browse_focused = false;
            s.browse_selected = false;
            s.bump();
            window.blur();
            cx.notify();
        }
    }

    pub(crate) fn clear_browse(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.browse_query.clear();
        s.browse_selected = false;
        s.browse_searched.clear();
        s.browse_results.clear();
        s.browse_error = None;
        s.browse_searching = false;
        // Invalida buscas ainda na fila.
        s.browse_search_id += 1;
        s.bump();
        cx.notify();
    }

    pub(crate) fn on_browse_key(
        &self,
        event: &open_gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ks = &event.keystroke;
        // Ctrl+A / Cmd+A seleciona tudo (sem disparar busca).
        if (ks.modifiers.control || ks.modifiers.platform)
            && !ks.modifiers.alt
            && ks.key.eq_ignore_ascii_case("a")
        {
            let mut s = self.shared.lock().unwrap();
            if !s.browse_query.is_empty() {
                s.browse_selected = true;
                s.bump();
                cx.notify();
            }
            return;
        }
        let mut s = self.shared.lock().unwrap();
        match ks.key.as_str() {
            "backspace" => {
                if s.browse_selected {
                    s.browse_query.clear();
                    s.browse_selected = false;
                } else {
                    s.browse_query.pop();
                }
            }
            "escape" => {
                if s.browse_selected {
                    // Primeiro Esc só solta a seleção.
                    s.browse_selected = false;
                } else if !s.browse_query.is_empty() {
                    s.browse_query.clear();
                    s.browse_searched.clear();
                    s.browse_results.clear();
                    s.browse_error = None;
                    s.browse_searching = false;
                    s.browse_search_id += 1;
                } else {
                    s.browse_focused = false;
                    s.bump();
                    window.blur();
                    cx.notify();
                    return;
                }
            }
            "enter" => {
                // A busca só acontece no Enter, com o input focado.
                s.browse_selected = false;
                s.bump();
                drop(s);
                self.queue_browse_search();
                cx.notify();
                return;
            }
            _ => {
                if !ks.modifiers.control && !ks.modifiers.alt && !ks.modifiers.platform {
                    if let Some(ch) = ks.key_char.clone() {
                        if ch.chars().count() == 1 {
                            if s.browse_selected {
                                s.browse_query = ch;
                                s.browse_selected = false;
                            } else {
                                s.browse_query.push_str(&ch);
                            }
                        } else {
                            return;
                        }
                    } else {
                        // Setas e cia: soltam a seleção sem mexer no texto.
                        if s.browse_selected {
                            s.browse_selected = false;
                        } else {
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
        }
        s.bump();
        cx.notify();
    }

    /// Dispara a busca (só via Enter). Respostas antigas descartadas
    /// via `browse_search_id` (dois Enters rápidos).
    fn queue_browse_search(&self) {
        let shared = self.shared.clone();
        let (id, query) = {
            let mut s = shared.lock().unwrap();
            let query = s.browse_query.clone();
            if query.trim().is_empty() {
                s.browse_results.clear();
                s.browse_error = None;
                s.browse_searching = false;
                s.browse_search_id += 1;
                s.browse_searched.clear();
                s.bump();
                return;
            }
            s.browse_search_id += 1;
            s.browse_searching = true;
            s.browse_error = None;
            // Marca a query submetida: só ela mostra "sem resultados".
            s.browse_searched = query.clone();
            s.bump();
            (s.browse_search_id, query)
        };
        std::thread::Builder::new()
            .name("klipp-browse".into())
            .spawn(move || {
                {
                    let s = shared.lock().unwrap();
                    if !s.browse_open || s.browse_search_id != id || s.browse_query != query {
                        return;
                    }
                }
                match backend::online::search(&query) {
                    Ok(results) => {
                        let mut s = shared.lock().unwrap();
                        if s.browse_search_id != id {
                            return;
                        }
                        s.browse_results = results;
                        s.browse_searching = false;
                        s.bump();
                    }
                    Err(err) => {
                        let mut s = shared.lock().unwrap();
                        if s.browse_search_id != id {
                            return;
                        }
                        s.browse_searching = false;
                        s.browse_error = Some(format!("{err}"));
                        s.bump();
                    }
                }
            })
            .ok();
    }

    /// Preview de um resultado: baixa o mp3 num temp e toca via `Engine`.
    /// Mesmo id já em preview: clique simples PARA; clique duplo ignora
    /// (senão o 2º clique do duplo-clique mataria o play recém-iniciado).
    pub(crate) fn preview_online(&self, id: String, clicks: usize, cx: &mut Context<Self>) {
        let found = self
            .shared
            .lock()
            .unwrap()
            .browse_results
            .iter()
            .find(|r| r.id == id)
            .cloned();
        let Some(item) = found else { return };
        {
            let s = self.shared.lock().unwrap();
            let active = s.browse_previewing.as_deref() == Some(&id)
                && (s.browse_preview_loading
                    || s.playing.as_deref() == Some(item.title.as_str()));
            if active {
                if clicks <= 1 {
                    drop(s);
                    self.engine.stop();
                    let mut s = self.shared.lock().unwrap();
                    s.browse_previewing = None;
                    s.browse_preview_loading = false;
                    s.bump();
                    cx.notify();
                }
                return;
            }
        }
        {
            let mut s = self.shared.lock().unwrap();
            s.browse_previewing = Some(id.clone());
            s.browse_preview_loading = true;
            s.bump();
        }
        cx.notify();
        let shared = self.shared.clone();
        let engine = self.engine.clone();
        std::thread::Builder::new()
            .name("klipp-preview".into())
            .spawn(move || {
                let outcome = (|| -> anyhow::Result<(std::path::PathBuf, String)> {
                    let bytes = backend::online::fetch_bytes(&item.mp3)?;
                    let path = backend::online::preview_path(&item.id, &item.mp3);
                    std::fs::write(&path, bytes)?;
                    Ok((path, item.title.clone()))
                })();
                match outcome {
                    Ok((path, title)) => {
                        let mut s = shared.lock().unwrap();
                        if s.browse_previewing.as_deref() != Some(&id) {
                            return;
                        }
                        s.browse_preview_loading = false;
                        s.browse_preview_started_ms = super::format::now_ms();
                        s.bump();
                        drop(s);
                        engine.play(path, title, shared);
                    }
                    Err(err) => {
                        let mut s = shared.lock().unwrap();
                        if s.browse_previewing.as_deref() == Some(&id) {
                            s.browse_previewing = None;
                        }
                        s.browse_preview_loading = false;
                        s.last_error = Some(format!("preview: {err}"));
                        s.bump();
                    }
                }
            })
            .ok();
    }

    /// Download de um resultado direto para a biblioteca local.
    pub(crate) fn download_online(&self, id: String, cx: &mut Context<Self>) {
        let found = self
            .shared
            .lock()
            .unwrap()
            .browse_results
            .iter()
            .find(|r| r.id == id)
            .cloned();
        let Some(item) = found else { return };
        {
            let mut s = self.shared.lock().unwrap();
            if s.browse_downloading.as_deref() == Some(&id) {
                return;
            }
            s.browse_downloading = Some(id.clone());
            s.bump();
        }
        cx.notify();
        let shared = self.shared.clone();
        std::thread::Builder::new()
            .name("klipp-download".into())
            .spawn(move || {
                match backend::online::download_to_library(&item.title, &item.mp3) {
                    Ok(name) => {
                        log::info!("baixado via browse: {name}");
                        backend::pick::refresh_library(&shared);
                        let mut s = shared.lock().unwrap();
                        if s.browse_downloading.as_deref() == Some(&id) {
                            s.browse_downloading = None;
                        }
                        s.bump();
                    }
                    Err(err) => {
                        let mut s = shared.lock().unwrap();
                        if s.browse_downloading.as_deref() == Some(&id) {
                            s.browse_downloading = None;
                        }
                        s.last_error = Some(format!("download: {err}"));
                        s.bump();
                    }
                }
            })
            .ok();
    }
}

#[allow(clippy::too_many_arguments)]
fn browse_results(
    lang: &str,
    query: &str,
    searched: &str,
    results: &[OnlineSound],
    sounds: &[Sound],
    searching: bool,
    error: Option<&str>,
    downloading: Option<&str>,
    previewing: Option<&str>,
    preview_loading: bool,
    playing: Option<&str>,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let body: open_gpui::AnyElement = if searching && results.is_empty() {
        div()
            .py_8()
            .flex()
            .justify_center()
            .text_sm()
            .text_color(theme::muted())
            .child(t(lang, "browse.searching"))
            .into_any_element()
    } else if let Some(err) = error {
        div()
            .py_8()
            .px_2()
            .flex()
            .justify_center()
            .text_sm()
            .text_color(theme::muted())
            .child(err.to_string())
            .into_any_element()
    } else if query.trim().is_empty() {
        div()
            .py_8()
            .flex()
            .justify_center()
            .text_sm()
            .text_color(theme::muted())
            .child(t(lang, "browse.hint"))
            .into_any_element()
    } else if query != searched {
        // Digitando após (ou antes de) um Enter: nada a declarar ainda.
        div()
            .py_8()
            .flex()
            .justify_center()
            .text_sm()
            .text_color(theme::muted())
            .child(t(lang, "browse.enter_hint"))
            .into_any_element()
    } else if results.is_empty() {
        div()
            .py_8()
            .px_2()
            .flex()
            .justify_center()
            .text_sm()
            .text_color(theme::muted())
            .child(t_fmt(lang, "browse.no_results", &[("query", query)]))
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .children(results.iter().map(|item| {
                let id = item.id.clone();
                let active = previewing == Some(item.id.as_str())
                    && (preview_loading || playing == Some(item.title.as_str()));
                let is_downloading = downloading == Some(item.id.as_str());
                // Já está na biblioteca (baixado antes ou nome igual)?
                // Mostra lixeira para apagar em vez de baixar de novo.
                let local = backend::online::library_match_name(sounds, &item.title);
                let action: open_gpui::AnyElement = match local {
                    Some(name) => div()
                        .id(format!("browse-del-{}", item.id))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            cx.stop_propagation();
                            this.delete_sound(name.clone(), cx)
                        }))
                        .p_2()
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme::card_hover()))
                        .child(icon("lucide-trash-2", 16.0, theme::danger_hover()))
                        .into_any_element(),
                    None => div()
                        .id(format!("browse-dl-{}", item.id))
                        .on_click(cx.listener({
                            let dl_id = item.id.clone();
                            move |this, _e, _w, cx| {
                                cx.stop_propagation();
                                this.download_online(dl_id.clone(), cx)
                            }
                        }))
                        .p_2()
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme::card_hover()))
                        .child(icon(
                            "lucide-download",
                            16.0,
                            if is_downloading {
                                theme::muted()
                            } else {
                                theme::accent()
                            },
                        ))
                        .into_any_element(),
                };
                div()
                    .id(format!("browse-row-{}", item.id))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .bg(theme::card())
                    .border_1()
                    .border_color(theme::border())
                    .rounded(px(10.0))
                    .child(
                        div()
                            .id(format!("browse-play-{id}"))
                            .on_click(cx.listener(move |this, event: &open_gpui::ClickEvent, _w, cx| {
                                cx.stop_propagation();
                                this.preview_online(id.clone(), event.click_count(), cx)
                            }))
                            .size(px(34.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .bg(theme::panel())
                            .border_1()
                            .border_color(theme::border())
                            .rounded(px(8.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::card_hover()))
                            .child(icon(
                                if active { "lucide-square" } else { "lucide-play" },
                                14.0,
                                if active { theme::accent() } else { theme::text() },
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(item.title.clone()),
                    )
                    .children(is_downloading.then(|| {
                        div()
                            .text_xs()
                            .text_color(theme::muted())
                            .flex_shrink_0()
                            .child(t(lang, "browse.downloading"))
                            .into_any_element()
                    }))
                    .child(action)
            }))
            .into_any_element()
    };
    div()
        .id("browse-scroll")
        .flex_1()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .px_5()
        .py_4()
        .child(body)
}
