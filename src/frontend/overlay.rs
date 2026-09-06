use std::sync::{Arc, Mutex};
use std::time::Duration;

use open_gpui::{
    div, img, px, transparent_black, Animation, AnimationExt, Bounds, Context, DisplayId,
    EntityId, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Render, RenderImage,
    Styled, Window, ease_out_quint, prelude::*,
};

use super::pie;
use super::theme;
use crate::core::state::Shared;

/// Janela overlay (pie selector). Lê/escreve apenas via [`Shared`].
///
/// Há uma instância por display (fullscreen layer-shell por output): o
/// compositor fixa cada surface num output, então um overlay único ficava
/// preso no monitor errado sem nunca receber o mouse do outro.
/// A âncora em [`Shared`] vive em coordenadas globais do desktop.
///
/// IMPORTANTE: `window.bounds()` de uma surface layer-shell sempre volta
/// `1x1@(0,0)` (o GPUI não sabe onde o compositor colocou a surface), então
/// a origem de cada janela vem do layout de displays (`display_id` + `cx`),
/// nunca de `window.bounds()`.
pub struct OverlayEntity {
    shared: Arc<Mutex<Shared>>,
    /// Base + destaque rasterizados por (versão, tema).
    pie_cache: Option<PieCache>,
    /// Output que esta janela cobre (None = transiente da largada).
    display: Option<DisplayId>,
    /// Entidades overlay de todas as janelas: mudar o estado numa precisa
    /// repintar as outras (cada janela só repinta a própria entidade).
    /// Ids mortos (janelas fechadas) são no-ops no `notify`.
    entities: Arc<Mutex<Vec<EntityId>>>,
    /// Renderiza o pie de fallback (sem âncora) nesta janela: só a do
    /// display primário, para não duplicar o pie em todos os monitores.
    fallback: bool,
}

/// Base do pie + destaque do hover. A base só muda com sons/tema (não com
/// hover), então o hover regenera só a fatia — sem o hitch do re-raster
/// completo.
struct PieCache {
    light: bool,
    names: Vec<String>,
    base: Arc<RenderImage>,
    highlight: Option<(usize, Arc<RenderImage>)>,
}

/// Origem global do display desta janela (`(0,0)` se desconhecido).
fn origin_of(cx: &mut Context<OverlayEntity>, display: Option<DisplayId>) -> (f32, f32) {
    cx.displays()
        .iter()
        .find(|d| Some(d.id()) == display)
        .map(|d| {
            let o = d.bounds().origin;
            (f32::from(o.x), f32::from(o.y))
        })
        .unwrap_or((0.0, 0.0))
}

/// Bounds globais do display desta janela, se conhecido.
fn display_bounds(
    cx: &mut Context<OverlayEntity>,
    display: Option<DisplayId>,
) -> Option<Bounds<Pixels>> {
    cx.displays()
        .iter()
        .find(|d| Some(d.id()) == display)
        .map(|d| d.bounds())
}

fn contains(bounds: Bounds<Pixels>, global: (f32, f32)) -> bool {
    let ox = f32::from(bounds.origin.x);
    let oy = f32::from(bounds.origin.y);
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    global.0 >= ox && global.0 < ox + w && global.1 >= oy && global.1 < oy + h
}

impl OverlayEntity {
    pub fn new(
        shared: Arc<Mutex<Shared>>,
        entities: Arc<Mutex<Vec<EntityId>>>,
        display: Option<DisplayId>,
        fallback: bool,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            shared,
            pie_cache: None,
            entities,
            display,
            fallback,
        }
    }

    /// Repinta os overlays das outras janelas (monitores).
    fn notify_siblings(&self, cx: &mut Context<Self>) {
        let ids = self.entities.lock().unwrap().clone();
        // `Context::notify` (0 args) esconde o `App::notify(id)` via Deref:
        // qualifica para notificar cada entidade nas suas janelas.
        let app: &mut open_gpui::App = std::ops::DerefMut::deref_mut(cx);
        for id in ids {
            app.notify(id);
        }
    }

    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        let mut shared = self.shared.lock().unwrap();
        if !active {
            shared.overlay_anchor = None;
            shared.anchor_needs_confirm = false;
            shared.overlay_fading = false;
            shared.pie_hovered = None;
            shared.center_hovered = false;
        }
        shared.overlay_active = active;
        drop(shared);
        cx.notify();
    }

    fn on_pointer_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        // Âncora + contagem sob um lock curto; geometria pura vive em `pie`.
        // Tudo em coordenadas globais para valer em qualquer monitor.
        // `event.position` é local da janela: soma a origem do display.
        let origin = origin_of(cx, self.display);
        let pos = (
            f32::from(event.position.x) + origin.0,
            f32::from(event.position.y) + origin.1,
        );
        let (anchor, count, active) = {
            let mut shared = self.shared.lock().unwrap();
            if !shared.overlay_active || shared.overlay_fading {
                return;
            }
            // Sem âncora exata (fallback): o primeiro evento real fixa a
            // posição do cursor.
            let anchor = if shared.anchor_needs_confirm || shared.overlay_anchor.is_none() {
                shared.anchor_needs_confirm = false;
                shared.overlay_anchor = Some(pos);
                shared.bump();
                log::info!("overlay âncora: ({:.0}, {:.0})", pos.0, pos.1);
                pos
            } else {
                shared.overlay_anchor.unwrap_or(pos)
            };
            (anchor, shared.favorite_sounds().len(), true)
        };
        let _ = active;

        // Centro só vale como opção com áudio tocando.
        let playing = self.shared.lock().unwrap().playing.is_some();
        let center = playing && pie::in_center(pos.0, pos.1, anchor);
        let hovered = if center {
            None
        } else {
            pie::hit_test(pos.0, pos.1, anchor, count).0
        };

        let mut shared = self.shared.lock().unwrap();
        if shared.pie_hovered != hovered || shared.center_hovered != center {
            shared.pie_hovered = hovered;
            shared.center_hovered = center;
            shared.bump();
        }
        self.notify_siblings(cx);
        cx.notify();
    }

    fn on_pointer_down(&mut self, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if event.button != MouseButton::Left {
            return;
        }
        let mut shared = self.shared.lock().unwrap();
        if !shared.overlay_active || shared.overlay_fading {
            return;
        }
        let origin = origin_of(cx, self.display);
        let pos = (
            f32::from(event.position.x) + origin.0,
            f32::from(event.position.y) + origin.1,
        );
        // Sem âncora (clique antes de qualquer mouse_move): usa o centro do
        // display em coords globais — o mesmo fallback do render.
        let anchor = shared.overlay_anchor.or_else(|| {
            display_bounds(cx, self.display).map(|b| {
                (
                    f32::from(b.origin.x) + f32::from(b.size.width) * 0.5,
                    f32::from(b.origin.y) + f32::from(b.size.height) * 0.5,
                )
            })
        });
        let count = shared.favorite_sounds().len();
        let playing = shared.playing.is_some();
        let (hovered, inside, center) = anchor
            .map(|a| {
                let (h, inside) = pie::hit_test(pos.0, pos.1, a, count);
                (h, inside, !inside && playing && pie::in_center(pos.0, pos.1, a))
            })
            .unwrap_or((None, false, false));

        if inside {
            if let Some(idx) = hovered {
                if let Some(sound) = shared.favorite_sounds().get(idx) {
                    shared.play_request = Some(sound.name.clone());
                }
            }
        } else if center {
            shared.stop_request = true;
        }
        // Fecha com fade-out rápido (~110ms): mantém montado, o tick
        // desmonta (ver `on_tick`).
        shared.overlay_fading = true;
        shared.overlay_fade_start = super::format::now_ms();
        shared.bump();
        drop(shared);
        self.notify_siblings(cx);
        cx.notify();
    }

    /// Base (por sons+tema) + destaque (por hover). Troca de hover
    /// regenera só a fatia — o re-raster completo dava o delay.
    fn pie_images(
        &mut self,
        light: bool,
        hovered: Option<usize>,
        names: &[String],
    ) -> Option<(Arc<RenderImage>, Option<(usize, Arc<RenderImage>)>)> {
        let count = names.len();
        // Hit de cache (base estável + mesmo destaque): sem re-raster.
        let cached = match &self.pie_cache {
            Some(cache) if cache.light == light && cache.names == names => {
                match (hovered, &cache.highlight) {
                    (Some(i), Some((j, img))) if i == *j => {
                        Some((cache.base.clone(), Some((i, img.clone()))))
                    }
                    (None, _) => Some((cache.base.clone(), None)),
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(hit) = cached {
            return Some(hit);
        }
        // Troca de hover com base pronta: regenera só a fatia.
        if let Some(base) = self
            .pie_cache
            .as_ref()
            .filter(|c| c.light == light && c.names == names)
            .map(|c| c.base.clone())
        {
            let highlight = hovered.and_then(|i| {
                pie::render_pie_highlight_image(count, i, light).map(|rgba| {
                    (
                        i,
                        Arc::new(RenderImage::new(smallvec::smallvec![
                            image::Frame::new(rgba)
                        ])),
                    )
                })
            });
            self.pie_cache = Some(PieCache {
                light,
                names: names.to_vec(),
                base: base.clone(),
                highlight: highlight.clone(),
            });
            return Some((base, highlight));
        }
        if !self.shared.lock().unwrap().overlay_active {
            return None;
        }
        let base = Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(
            pie::render_pie_image(count, light)?,
        )]));
        let highlight = match hovered {
            Some(i) => pie::render_pie_highlight_image(count, i, light).map(|rgba| {
                (
                    i,
                    Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(
                        rgba,
                    )])),
                )
            }),
            None => None,
        };
        self.pie_cache = Some(PieCache {
            light,
            names: names.to_vec(),
            base: base.clone(),
            highlight: highlight.clone(),
        });
        Some((base, highlight))
    }
}

impl Render for OverlayEntity {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (light, hovered, center_hovered, seq, fading, playing, slices, names, total) = {
            let shared = self.shared.lock().unwrap();
            let fav = shared.favorite_sounds();
            let slices: Vec<(String, String)> = fav
                .iter()
                .take(pie::PAGE)
                .map(|s| (super::format::sound_label(s), s.emoji.clone()))
                .collect();
            let names: Vec<String> = slices.iter().map(|(l, _)| l.clone()).collect();
            let total = fav.len();
            (
                theme::is_light(),
                shared.pie_hovered,
                shared.center_hovered,
                shared.overlay_seq,
                shared.overlay_fading,
                shared.playing.is_some(),
                slices,
                names,
                total,
            )
        };

        let root = div()
            .id("overlay-root")
            .size_full()
            .bg(transparent_black())
            .on_mouse_move(cx.listener(Self::on_pointer_move))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_pointer_down));

        let Some((base, highlight)) = self.pie_images(light, hovered, &names) else {
            return root;
        };

        let anchor_global = self.shared.lock().unwrap().overlay_anchor;
        // Com âncora: só a janela do monitor que contém a âncora desenha o
        // pie (as demais seguem transparentes, só capturando clique fora).
        // Sem âncora: só a janela primária mostra o fallback central até o
        // primeiro mouse_move fixar a âncora no cursor.
        // Usa o layout de displays: `window.bounds()` de layer-shell mente
        // (sempre 1x1@(0,0)).
        let local = match anchor_global {
            Some(g) => {
                let Some(bounds) = display_bounds(cx, self.display) else {
                    return root;
                };
                if !contains(bounds, g) {
                    return root;
                }
                let o = bounds.origin;
                (
                    g.0 - f32::from(o.x),
                    g.1 - f32::from(o.y),
                )
            }
            None => {
                if !self.fallback {
                    return root;
                }
                // Centro do próprio display (ou da janela, se display
                // desconhecido — caso transiente da largada).
                match display_bounds(cx, self.display) {
                    Some(b) => (
                        f32::from(b.size.width) * 0.5,
                        f32::from(b.size.height) * 0.5,
                    ),
                    None => {
                        let size = window.bounds().size;
                        (
                            f32::from(size.width) * 0.5,
                            f32::from(size.height) * 0.5,
                        )
                    }
                }
            }
        };

        let mut pie_box = div()
            .absolute()
            .left(px(local.0 - pie::PIE / 2.0))
            .top(px(local.1 - pie::PIE / 2.0))
            .w(px(pie::PIE))
            .h(px(pie::PIE))
            .child(img(base).size_full());
        // Cursor de mão sobre fatia ou botão central.
        if hovered.is_some() || center_hovered {
            pie_box = pie_box.cursor_pointer();
        }
        // Destaque do hover numa camada própria com crossfade rápido.
        if let Some((idx, hl)) = highlight {
            pie_box = pie_box.child(
                div()
                    .absolute()
                    .inset_0()
                    .child(img(hl).size_full())
                    .with_animation(
                        format!("pie-hover-{seq}-{idx}"),
                        Animation::new(Duration::from_millis(120))
                            .with_easing(ease_out_quint()),
                        |el, delta| el.opacity(delta),
                    ),
            );
        }
        // Rótulos como divs GPUI (tema + emoji de verdade, sem contorno):
        // emoji do som (ou ♪) + nome curto sobre cada fatia. O emoji herda
        // a cor do texto (branco no hover).
        let white = open_gpui::rgb(0xffffff);
        let n = slices.len();
        for (i, (label, emoji)) in slices.iter().enumerate() {
            let (mx, my) = pie::slice_mid(i, n);
            let is_hovered = hovered == Some(i);
            pie_box = pie_box.child(
                div()
                    .absolute()
                    .left(px(mx - 55.0))
                    .top(px(my - 32.0))
                    .w(px(110.0))
                    .h(px(64.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .text_color(if is_hovered { white } else { theme::text() })
                    .child(super::library::pad_emoji(emoji, px(26.0), &self.shared))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(open_gpui::FontWeight::SEMIBOLD)
                            .truncate()
                            .child(pie::short_label(label)),
                    ),
            );
        }
        // Centro: nome + (■ só com áudio tocando) + contagem. O hover
        // preenche o fundo com accent (igual às fatias), sem borda.
        let stop_fg = if !playing {
            theme::muted()
        } else if center_hovered {
            open_gpui::rgb(0xffffff)
        } else {
            theme::accent()
        };
        if center_hovered && playing {
            pie_box = pie_box.child(
                div()
                    .absolute()
                    .left(px(pie::CENTER - 69.0))
                    .top(px(pie::CENTER - 69.0))
                    .w(px(138.0))
                    .h(px(138.0))
                    .rounded(px(999.0))
                    .bg(theme::accent())
                    .with_animation(
                        format!("pie-center-{seq}"),
                        Animation::new(Duration::from_millis(120))
                            .with_easing(ease_out_quint()),
                        |el, delta| el.opacity(delta),
                    ),
            );
        }
        pie_box = pie_box.child(
            div()
                .absolute()
                .left(px(pie::CENTER - 69.0))
                .top(px(pie::CENTER - 69.0))
                .w(px(138.0))
                .h(px(138.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(open_gpui::FontWeight::BOLD)
                        .text_color(if center_hovered && playing {
                            open_gpui::rgb(0xffffff)
                        } else {
                            theme::text()
                        })
                        .child("Klipp".to_string()),
                )
                .children(playing.then(|| {
                    div()
                        .text_xl()
                        .text_color(stop_fg)
                        .child("■".to_string())
                }))
                .child(
                    div()
                        .text_xs()
                        .text_color(if center_hovered && playing {
                            open_gpui::rgb(0xffffff)
                        } else {
                            theme::muted()
                        })
                        .child(format!("{total}")),
                ),
        );
        // Fade-in ao mostrar (~150ms), fade-out rápido ao esconder (~110ms).
        // Por último (anima o conjunto); chaves com `seq`: cada ativação
        // remonta e toca do zero.
        let pie_box = if fading {
            pie_box.with_animation(
                format!("pie-hide-{seq}"),
                Animation::new(Duration::from_millis(110))
                    .with_easing(ease_out_quint()),
                |el, delta| el.opacity(1.0 - delta),
            )
        } else {
            pie_box.with_animation(
                format!("pie-show-{seq}"),
                Animation::new(Duration::from_millis(150))
                    .with_easing(ease_out_quint()),
                |el, delta| el.opacity(delta),
            )
        };

        root.child(pie_box)
    }
}
