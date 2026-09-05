use std::sync::{Arc, Mutex};

use open_gpui::{
    div, px, svg, transparent_black, Context, MouseButton, MouseDownEvent, MouseMoveEvent, Render,
    Styled, Window,
    prelude::*,
};

use super::assets::AppAssets;
use super::pie;
use crate::core::state::Shared;

/// Janela overlay (pie selector). Lê/escreve apenas via [`Shared`].
pub struct OverlayEntity {
    shared: Arc<Mutex<Shared>>,
    svgs: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl OverlayEntity {
    pub fn new(
        shared: Arc<Mutex<Shared>>,
        assets: &Arc<AppAssets>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            shared,
            svgs: assets.svgs.clone(),
        }
    }

    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        let mut shared = self.shared.lock().unwrap();
        if !active {
            shared.overlay_anchor = None;
            shared.pie_hovered = None;
        }
        shared.overlay_active = active;
        drop(shared);
        cx.notify();
    }

    fn on_pointer_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        // Âncora + contagem sob um lock curto; geometria pura vive em `pie`.
        let (anchor, count, active) = {
            let mut shared = self.shared.lock().unwrap();
            if !shared.overlay_active {
                return;
            }
            let anchor = shared
                .overlay_anchor
                .unwrap_or((f32::from(event.position.x), f32::from(event.position.y)));
            shared.overlay_anchor = Some(anchor);
            (anchor, shared.sounds.len(), true)
        };
        let _ = active;

        let hovered =
            pie::hit_test(f32::from(event.position.x), f32::from(event.position.y), anchor, count).0;

        let mut shared = self.shared.lock().unwrap();
        if shared.pie_hovered != hovered {
            shared.pie_hovered = hovered;
            shared.bump();
        }
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
        let anchor = shared.overlay_anchor;
        let count = shared.sounds.len();
        let (hovered, inside) = anchor
            .map(|a| {
                pie::hit_test(
                    f32::from(event.position.x),
                    f32::from(event.position.y),
                    a,
                    count,
                )
            })
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
        shared.pie_hovered = None;
        shared.bump();
        cx.notify();
    }

    fn pie_svg(&self, version: u64) -> Option<(String, String)> {
        let shared = self.shared.lock().unwrap();
        if !shared.overlay_active {
            return None;
        }
        let hovered = shared.pie_hovered;
        let names: Vec<String> = shared
            .sounds
            .iter()
            .take(pie::PAGE)
            .map(super::format::sound_label)
            .collect();
        drop(shared);

        let key = format!("klipp-pie-v{version}");
        let svg = pie::gen_pie_svg(&names, hovered);
        Some((key, svg))
    }
}

impl Render for OverlayEntity {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let version = self.shared.lock().unwrap().version;

        let root = div()
            .id("overlay-root")
            .size_full()
            .bg(transparent_black())
            .on_mouse_move(cx.listener(Self::on_pointer_move))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_pointer_down));

        let Some((key, svg_text)) = self.pie_svg(version) else {
            return root;
        };
        self.svgs.lock().unwrap().insert(key.clone(), svg_text);

        let anchor = self.shared.lock().unwrap().overlay_anchor;
        let Some((x, y)) = anchor else {
            return root;
        };

        root.child(
            div()
                .absolute()
                .left(px(x - pie::PIE / 2.0))
                .top(px(y - pie::PIE / 2.0))
                .w(px(pie::PIE))
                .h(px(pie::PIE))
                .child(svg().path(key).size_full().text_color(open_gpui::rgb(0xffffff))),
        )
    }
}
