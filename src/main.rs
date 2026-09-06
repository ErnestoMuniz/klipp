mod backend;
mod core;
mod frontend;

use std::sync::{Arc, Mutex};

use open_gpui::{px, size, App, Bounds, WindowBounds, WindowDecorations, WindowOptions};
use open_gpui::prelude::*;
use open_gpui_platform::application;

use crate::backend::{AudioGraph, Engine};
use crate::core::state::Shared;
use crate::frontend::{AppAssets, MainWindow};

/// Bootstrap fino: monta services do backend + estado compartilhado,
/// depois entrega o resto para o frontend (GPUI).
fn main() {
    // Spans internos do GPUI (`sum_tree::seek_internal` etc.) disparam a cada
    // layout — com repaint a 15fps viram spam infinito. Silencia alvos barulhentos.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        "info,tracing::span=off,open_gpui_sum_tree=off,gpui_sum_tree=off,sum_tree=off,zbus::proxy=error,usvg=error",
    ))
    .init();

    let assets = AppAssets::new();
    let engine = Engine::new();
    let graph = Arc::new(Mutex::new(AudioGraph::new()));
    let settings = core::settings::load();
    let shared = Arc::new(Mutex::new(Shared::new(&settings)));

    let (shared2, engine2, graph2, assets2) =
        (shared.clone(), engine.clone(), graph.clone(), assets.clone());
    application()
        .with_assets(AppAssets {
            svgs: assets2.svgs.clone(),
        })
        .run(move |cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(900.0), px(720.0)), cx);
            let _window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        // Sem titlebar nativa: o app desenha a própria
                        // (`frontend::main_window::titlebar`).
                        // Com `Client` o compositor não dá bordas de resize,
                        // então o app desenha as próprias alças (ver
                        // `resize_handles` no render).
                        titlebar: None,
                        window_decorations: Some(WindowDecorations::Client),
                        window_min_size: Some(size(px(640.0), px(480.0))),
                        app_id: Some("klipp".into()),
                        ..Default::default()
                    },
                    |_window, cx| {
                        cx.new(|cx| {
                            MainWindow::new(shared2.clone(), engine2.clone(), graph2.clone(), &assets2, cx)
                        })
                    },
                )
                .map_err(|err| log::error!("não foi possível abrir a janela principal: {err}"));
        });

    // App fechou: desfaz o grafo de áudio e para o playback.
    engine.shutdown();
    graph.lock().unwrap().cleanup();
}
