use open_gpui::{Context, IntoElement, MouseButton, div, prelude::*, px};

use super::format::{format_clock, progress_fraction, rgb_dark};
use super::icons::icon;
use super::main_window::MainWindow;
use super::theme;
use crate::core::i18n::t;
use crate::core::state::WAVEFORM_BARS;

/// Largura útil da barra de progresso (player de 680px menos `px_5` dos lados).
const PROGRESS_W: f32 = 640.0;
/// Largura de cada barra da forma de onda (128 × 5px = 640px, sem vãos).
const BAR_W: f32 = PROGRESS_W / WAVEFORM_BARS as f32;
/// Altura da área da forma de onda; a barra vai de 3px ao teto.
const WAVE_H: f32 = 30.0;

impl MainWindow {
    pub(crate) fn bottom_stack(
        &self,
        playing: Option<&String>,
        playing_label: Option<&str>,
        volume: f32,
        muted: bool,
        playback: Option<(f32, f32)>,
        peaks: &[f32],
        paused: bool,
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
            .child(self.player_bar(playing, playing_label, volume, muted, playback, peaks, paused, lang, cx))
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
        playback: Option<(f32, f32)>,
        peaks: &[f32],
        paused: bool,
        lang: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Sem estado nem placeholders antigos: tocando mostra o nome do
        // áudio, parado mostra um placeholder neutro.
        let track_label = playing_label
            .map(str::to_string)
            .unwrap_or_else(|| t(lang, "player.idle"));
        // Tocando: pause. Pausado ou parado: play.
        let toggle_icon = if playing.is_some() && !paused {
            "lucide-pause"
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
            .flex_col()
            .gap_2()
            .bg(theme::panel())
            .border_1()
            .border_color(theme::border())
            .rounded(px(14.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
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
                            .child(
                                div()
                                    .w(px(180.0))
                                    .text_sm()
                                    .truncate()
                                    .text_color(if playing.is_some() {
                                        theme::text()
                                    } else {
                                        theme::muted()
                                    })
                                    .child(track_label),
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
                    ),
            )
            .child(self.progress_section(playback, peaks, cx))
    }

    /// Forma de onda clicável: 128 barras com a altura do pico de cada
    /// trecho (estática por play — só a cor muda com o progresso).
    /// Trechos já tocados em destaque, resto apagado; clique/arrasto faz
    /// seek. Sem playback mostra `0:00 / 0:00` com barras chapadas.
    fn progress_section(
        &self,
        playback: Option<(f32, f32)>,
        peaks: &[f32],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (elapsed, duration, frac, enabled) = match playback {
            Some((e, d)) if d > 0.0 => (e, d, progress_fraction(e, d), true),
            _ => (0.0, 0.0, 0.0, false),
        };
        let bars: Vec<open_gpui::AnyElement> = (0..WAVEFORM_BARS)
            .map(|i| {
                // Pico do trecho ou base chapada quando parado.
                let peak = if enabled && peaks.len() == WAVEFORM_BARS {
                    peaks[i].clamp(0.0, 1.0)
                } else {
                    0.06
                };
                let h = 3.0 + peak * (WAVE_H - 3.0);
                let played = enabled && (i as f32 + 0.5) / WAVEFORM_BARS as f32 <= frac;
                let color = if played {
                    theme::accent()
                } else {
                    theme::border()
                };
                // Retas no meio: só a primeira arredonda à esquerda e a
                // última à direita (bloco único, sem vãos).
                let bar = div().w(px(BAR_W)).h(px(h)).bg(color);
                let bar = if i == 0 {
                    bar.rounded_l(px(2.0))
                } else if i == WAVEFORM_BARS - 1 {
                    bar.rounded_r(px(2.0))
                } else {
                    bar
                };
                bar.into_any_element()
            })
            .collect();
        let track = self.progress_track.clone();

        let slider = div()
            .id("progress-slider")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(
                    |this, event: &open_gpui::MouseDownEvent, _window, cx| {
                        this.on_progress_down(f32::from(event.position.x), cx);
                    },
                ),
            )
            .w(px(PROGRESS_W))
            .h(px(WAVE_H))
            .flex()
            .flex_row()
            .items_center();
        // Só clicável com áudio: sem duração o cursor vira padrão.
        let slider = if enabled {
            slider.cursor_pointer()
        } else {
            slider
        };

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                open_gpui::measured_element(
                    "progress-track",
                    slider.children(bars),
                    move |_id, bounds, _gid, _window, _cx| {
                        *track.lock().unwrap() = Some(bounds);
                    },
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .text_color(theme::muted())
                    .child(format_clock(elapsed))
                    .child(format_clock(duration)),
            )
    }
}
