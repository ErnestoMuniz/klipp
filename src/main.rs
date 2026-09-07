mod backend;
mod core;
mod frontend;

use std::sync::{Arc, Mutex, OnceLock};

use open_gpui::{px, size, App, Bounds, QuitMode, WindowBounds, WindowDecorations, WindowHandle, WindowOptions};
use open_gpui::prelude::*;
use open_gpui_platform::application;

use crate::backend::tray::TrayEvent;
use crate::backend::{AudioGraph, Engine};
use crate::core::state::Shared;
use crate::frontend::{AppAssets, MainWindow};

/// Handle da janela principal: o tray foca a existente ou abre outra
/// (sem duplicar) quando a anterior foi fechada.
static MAIN_WINDOW: OnceLock<Mutex<Option<WindowHandle<MainWindow>>>> = OnceLock::new();

fn main_window_handle() -> &'static Mutex<Option<WindowHandle<MainWindow>>> {
    MAIN_WINDOW.get_or_init(|| Mutex::new(None))
}

fn open_main_window(
    app: &mut App,
    shared: &Arc<Mutex<Shared>>,
    engine: &Arc<Engine>,
    graph: &Arc<Mutex<AudioGraph>>,
    assets: &Arc<AppAssets>,
) {
    let bounds = Bounds::centered(None, size(px(900.0), px(720.0)), app);
    match app.open_window(
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
            app_id: Some("io.github.ErnestoMuniz.Klipp".into()),
            ..Default::default()
        },
        |_window, cx| {
            cx.new(|cx| {
                MainWindow::new(shared.clone(), engine.clone(), graph.clone(), assets, cx)
            })
        },
    ) {
        Ok(handle) => *main_window_handle().lock().unwrap() = Some(handle),
        Err(err) => log::error!("não foi possível abrir a janela principal: {err}"),
    }
}

/// Pedidos do tray icon, executados na UI thread.
fn handle_tray_event(
    app: &mut App,
    event: TrayEvent,
    shared: &Arc<Mutex<Shared>>,
    engine: &Arc<Engine>,
    graph: &Arc<Mutex<AudioGraph>>,
    assets: &Arc<AppAssets>,
) {
    match event {
        TrayEvent::Toggle => {
            // Janela viva: só traz ao frente. Fechada: abre outra.
            let focused = main_window_handle()
                .lock()
                .unwrap()
                .clone()
                .and_then(|h| {
                    h.update(app, |_, window, _| window.activate_window())
                        .ok()
                })
                .is_some();
            if !focused {
                open_main_window(app, shared, engine, graph, assets);
            }
        }
        TrayEvent::Quit => app.quit(),
    }
}

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
            // Com o tray, fechar a janela não encerra: a saída é sempre
            // explícita (X com a opção desligada, Alt+F4 idem, "Quit"
            // no menu do tray). Ver `MainWindow::request_close`.
            cx.set_quit_mode(QuitMode::Explicit);
            open_main_window(cx, &shared2, &engine2, &graph2, &assets2);
            // Consome os pedidos do tray icon na UI thread (~10Hz).
            // (Executor de foreground: o futuro segura `AsyncApp`, que
            // não é `Send` e não pode ir ao executor de background.)
            let app = cx.to_async();
            let (shared3, engine3, graph3, assets3) = (
                shared2.clone(),
                engine2.clone(),
                graph2.clone(),
                assets2.clone(),
            );
            cx.foreground_executor()
                .spawn(async move {
                    loop {
                        app.background_executor()
                            .timer(std::time::Duration::from_millis(100))
                            .await;
                        let events = crate::backend::tray::drain_events();
                        if events.is_empty() {
                            continue;
                        }
                        app.update(|app| {
                            for event in events {
                                handle_tray_event(
                                    app, event, &shared3, &engine3, &graph3, &assets3,
                                );
                            }
                        });
                    }
                })
                .detach();
        });

    // App fechou: desfaz o grafo de áudio e para o playback.
    engine.shutdown();
    graph.lock().unwrap().cleanup();
}
