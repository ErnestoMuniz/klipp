use std::sync::{Arc, Mutex};

use open_gpui::{
    div, img, px, transparent_black, Bounds, Context, DisplayId, EntityId, MouseButton,
    MouseDownEvent, MouseMoveEvent, Pixels, Render, RenderImage, Styled, Window,
    prelude::*,
};

use super::pie;
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
    /// Render do pie rasterizado (cores fiéis) por versão do estado.
    pie_cache: Option<(u64, Arc<RenderImage>)>,
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
            shared.anchor_x = None;
            shared.pie_hovered = None;
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
            if !shared.overlay_active {
                return;
            }
            // Âncora do XWayland é aproximada: o primeiro evento real do
            // Wayland corrige para a posição exata do cursor e aprende o
            // offset X→Wayland (próximas ativações já abrem no lugar).
            let anchor = if shared.anchor_needs_confirm || shared.overlay_anchor.is_none() {
                shared.anchor_needs_confirm = false;
                if let Some(x) = shared.anchor_x.take() {
                    // Só aprende o offset se o cursor ficou parado (perto do
                    // X): se o usuário andou até aqui, a diferença é
                    // movimento, não erro sistemático.
                    let (dx, dy) = (pos.0 - x.0, pos.1 - x.1);
                    if dx.hypot(dy) < 250.0 {
                        shared.cursor_calib = Some((dx, dy));
                        log::info!("overlay calib: ({dx:.0}, {dy:.0})");
                    } else {
                        log::info!("overlay sem calibrar (moveu {dx:.0}, {dy:.0})");
                    }
                }
                shared.overlay_anchor = Some(pos);
                shared.bump();
                log::info!("overlay âncora: ({:.0}, {:.0})", pos.0, pos.1);
                pos
            } else {
                shared.overlay_anchor.unwrap_or(pos)
            };
            (anchor, shared.sounds.len(), true)
        };
        let _ = active;

        let hovered = pie::hit_test(pos.0, pos.1, anchor, count).0;

        let mut shared = self.shared.lock().unwrap();
        if shared.pie_hovered != hovered {
            shared.pie_hovered = hovered;
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
        if !shared.overlay_active {
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
        let count = shared.sounds.len();
        let (hovered, inside) = anchor
            .map(|a| pie::hit_test(pos.0, pos.1, a, count))
            .unwrap_or((None, false));

        if inside {
            if let Some(idx) = hovered {
                if let Some(sound) = shared.sounds.get(idx) {
                    shared.play_request = Some(sound.name.clone());
                }
            }
        }
        shared.overlay_active = false;
        shared.overlay_anchor = None;
        shared.anchor_needs_confirm = false;
        shared.anchor_x = None;
        shared.pie_hovered = None;
        shared.bump();
        drop(shared);
        self.notify_siblings(cx);
        cx.notify();
    }

    fn pie_image(&mut self, version: u64) -> Option<Arc<RenderImage>> {
        if let Some((v, image)) = &self.pie_cache {
            if *v == version {
                return Some(image.clone());
            }
        }
        let (hovered, names, active) = {
            let shared = self.shared.lock().unwrap();
            (
                shared.pie_hovered,
                shared
                    .sounds
                    .iter()
                    .take(pie::PAGE)
                    .map(super::format::sound_label)
                    .collect::<Vec<_>>(),
                shared.overlay_active,
            )
        };
        if !active {
            return None;
        }
        let rgba = pie::render_pie_image(&names, hovered)?;
        let frame = image::Frame::new(rgba);
        let rendered = Arc::new(RenderImage::new(smallvec::smallvec![frame]));
        self.pie_cache = Some((version, rendered.clone()));
        Some(rendered)
    }
}

impl Render for OverlayEntity {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let version = self.shared.lock().unwrap().version;

        let root = div()
            .id("overlay-root")
            .size_full()
            .bg(transparent_black())
            .on_mouse_move(cx.listener(Self::on_pointer_move))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_pointer_down));

        let Some(image) = self.pie_image(version) else {
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

        root.child(
            div()
                .absolute()
                .left(px(local.0 - pie::PIE / 2.0))
                .top(px(local.1 - pie::PIE / 2.0))
                .w(px(pie::PIE))
                .h(px(pie::PIE))
                .child(img(image).size_full()),
        )
    }
}
