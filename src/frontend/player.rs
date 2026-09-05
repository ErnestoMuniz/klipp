use open_gpui::{Context, IntoElement, MouseButton, div, prelude::*, px};

use super::format::{now_ms, rgb_dark};
use super::icons::icon;
use super::main_window::MainWindow;
use super::theme;
use crate::core::i18n::t;

impl MainWindow {
    pub(crate) fn bottom_stack(
        &self,
        playing: Option<&String>,
        playing_label: Option<&str>,
        volume: f32,
        muted: bool,
        lang: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .relative()
            .px_4()
            .pt_2()
            .pb_4()
            .flex()
            .flex_col()
            .items_center()
            .child(self.player_bar(playing, playing_label, volume, muted, lang, cx))
            .child(
                div()
                    .absolute()
                    .right(px(20.0))
                    .bottom(px(88.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .id("open-folder")
                            .on_click(cx.listener(|this, _e, _w, _cx| this.open_sounds_folder()))
                            .size(px(44.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme::card())
                            .border_1()
                            .border_color(theme::border())
                            .rounded(px(999.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::card_hover()))
                            .text_color(theme::text())
                            .child(icon("lucide-folder-open", 20.0, theme::text())),
                    )
                    .child(
                        div()
                            .id("fab-add")
                            .on_click(cx.listener(|this, _e, _w, cx| this.pick_files(cx)),
                            )
                            .size(px(52.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme::accent())
                            .rounded(px(999.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::accent_hover()))
                            .text_color(rgb_dark())
                            .child(icon("lucide-plus", 22.0, rgb_dark())),
                    ),
            )
    }

    fn player_bar(
        &self,
        playing: Option<&String>,
        playing_label: Option<&str>,
        volume: f32,
        muted: bool,
        lang: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (state_label, track_label) = match playing {
            Some(_) => (
                t(lang, "player.playing").to_uppercase(),
                playing_label.unwrap_or(&t(lang, "player.ready")).to_string(),
            ),
            None => (
                t(lang, "player.stopped").to_uppercase(),
                t(lang, "player.ready"),
            ),
        };
        let toggle_icon = if playing.is_some() {
            "lucide-square"
        } else {
            "lucide-play"
        };
        // Mudo zera o que a barra mostra (o `volume` real é preservado).
        let shown = if muted { 0.0 } else { volume.clamp(0.0, 1.0) };
        // Trilho fixo de 200px: o preenchimento acompanha o volume real.
        let fill_w = 200.0 * shown;
        let rest_w = 200.0 - fill_w;
        let vol_icon = if muted {
            "lucide-volume-x"
        } else {
            "lucide-volume-2"
        };
        let track = self.vol_track.clone();

        div()
            .w(px(680.0))
            .px_5()
            .py_3()
            .flex()
            .items_center()
            .justify_between()
            .bg(theme::panel())
            .border_1()
            .border_color(theme::border())
            .rounded(px(14.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .id("player-toggle")
                            .on_click(cx.listener(|this, _e, _w, cx| this.toggle_play(cx)),
                            )
                            .p_1()
                            .rounded(px(8.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::card_hover()))
                            .text_color(theme::muted())
                            .child(icon(toggle_icon, 20.0, theme::muted())),
                    )
                    .child(eq_bars(playing.is_some()))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .w(px(180.0))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme::muted())
                                    .child(state_label),
                            )
                            .child(div().text_sm().truncate().child(track_label)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(div().w(px(1.0)).h(px(32.0)).bg(theme::border()))
                    .child(
                        div()
                            .id("vol-mute")
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                cx.stop_propagation();
                                this.toggle_mute(cx)
                            }))
                            .p_1()
                            .rounded(px(8.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::card_hover()))
                            .text_color(theme::muted())
                            .child(icon(vol_icon, 18.0, theme::muted())),
                    )
                    .child(
                        open_gpui::measured_element(
                            "vol-track",
                            div()
                                .id("vol-slider")
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(
                                        |this, event: &open_gpui::MouseDownEvent, _window, cx| {
                                            this.on_vol_down(
                                                f32::from(event.position.x),
                                                cx,
                                            );
                                        },
                                    ),
                                )
                                .w(px(212.0))
                                .h(px(20.0))
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .rounded(px(999.0))
                                .child(
                                    div()
                                        .w(px(fill_w))
                                        .h(px(4.0))
                                        .bg(theme::accent())
                                        .rounded(px(999.0)),
                                )
                                .child(
                                    div()
                                        .size(px(12.0))
                                        .bg(theme::accent())
                                        .rounded(px(999.0)),
                                )
                                .child(
                                    div()
                                        .w(px(rest_w))
                                        .h(px(4.0))
                                        .bg(theme::border())
                                        .rounded(px(999.0)),
                                ),
                            move |_id, bounds, _gid, _window, _cx| {
                                *track.lock().unwrap() = Some(bounds);
                            },
                        ),
                    )
                    .child(
                        div()
                            .w(px(44.0))
                            .text_sm()
                            .text_right()
                            .child(format!("{:.0}%", shown * 100.0)),
                    ),
            )
    }
}

/// Equalizador do player: 4 barrinhas azuis oscilando enquanto toca,
/// pequenas e cinzas quando parado.
/// (Tempo em f64: o timestamp em ms não cabe num f32 sem quantizar.)
fn eq_bars(playing: bool) -> impl IntoElement {
    let t = now_ms() as f64 / 1000.0;
    div().flex().flex_row().items_center().gap_1().children(
        (0..4)
            .map(|i| {
                let h = if playing {
                    let phase = t * 5.2 + i as f64 * 1.5;
                    (5.0 + 9.0 * (0.5 + 0.5 * phase.sin())) as f32
                } else {
                    4.0
                };
                let c = if playing {
                    theme::accent()
                } else {
                    theme::border()
                };
                div().w(px(3.0)).h(px(h)).bg(c).rounded(px(999.0))
            })
            .collect::<Vec<_>>(),
    )
}
