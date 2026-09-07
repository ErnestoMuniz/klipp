use std::sync::{Arc, Mutex};

use open_gpui::{
    Context, CursorStyle, FocusHandle, IntoElement, MouseButton, Render, ResizeEdge, Styled,
    Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowHandle, WindowKind,
    WindowOptions, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    prelude::*,
    px, size,
};

use super::assets::AppAssets;
use super::format::{display_name, sound_label};
use super::library::{drop_overlay, empty_library, no_results};
use super::overlay::OverlayEntity;
use super::theme;
use super::titlebar::titlebar;
use super::ui::{hint_banner, status_banner};
use crate::backend::{self, AudioGraph, Engine};
use crate::core::i18n::{t, t_fmt};
use crate::core::state::{Shared, Sound};

/// Campo de texto customizado (inputs desenhados à mão, sem seleção nativa).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextField {
    Search,
    Browse,
    EditorName,
    EditorSearch,
}

/// Foco exclusivo entre os 4 inputs: focar um apaga os outros,
/// senão dois inputs parecem selecionados ao mesmo tempo.
pub(crate) fn focus_field(s: &mut Shared, field: TextField) {
    s.search_focused = field == TextField::Search;
    s.browse_focused = field == TextField::Browse;
    s.editor_name_focused = field == TextField::EditorName;
    s.editor_search_focused = field == TextField::EditorSearch;
}

/// Janela principal. Orquestra services do backend (audio/shortcut/playback)
/// e re-renderiza quando `Shared::version` muda (poll a cada ~66ms ≈ 15fps).
pub struct MainWindow {
    pub(crate) shared: Arc<Mutex<Shared>>,
    pub(crate) engine: Arc<Engine>,
    graph: Arc<Mutex<AudioGraph>>,
    overlay: Arc<Mutex<Vec<(WindowHandle<OverlayEntity>, Option<open_gpui::DisplayId>)>>>,
    /// Entidades overlay (uma por janela): para repintar todos os monitores
    /// quando o estado muda numa janela.
    overlay_ids: Arc<Mutex<Vec<open_gpui::EntityId>>>,
    pub(crate) search_focus: FocusHandle,
    pub(crate) editor_focus: FocusHandle,
    pub(crate) editor_search_focus: FocusHandle,
    pub(crate) browse_focus: FocusHandle,
    /// Card sob o mouse (para revelar as ações). Estado local da view.
    pub(crate) hovered_card: Option<usize>,
    /// Arrastar no slider de volume: botão segurado após pressionar a trilha.
    vol_dragging: bool,
    /// Retângulo da trilha do slider (medido a cada frame): permite mapear
    /// o clique em px de janela para fração 0–1 (pulo imediato).
    pub(crate) vol_track: Arc<Mutex<Option<open_gpui::Bounds<open_gpui::Pixels>>>>,
    /// Arrastar na barra de progresso: mesmo padrão do volume.
    progress_dragging: bool,
    /// Retângulo da trilha de progresso (medido a cada frame).
    pub(crate) progress_track: Arc<Mutex<Option<open_gpui::Bounds<open_gpui::Pixels>>>>,
    /// Arrastar para selecionar texto nos inputs: (campo, x inicial em px).
    /// Seleção dos inputs custom é tudo-ou-nada (sem cursor): arrasto além
    /// de 4px ou duplo-clique seleciona o texto todo.
    text_drag: Option<(TextField, f32)>,
    last_ver: u64,
    last_overlay_active: bool,
    /// `on_window_should_close` registrado (uma vez por janela).
    close_hooked: bool,
}

impl MainWindow {
    pub fn new(
        shared: Arc<Mutex<Shared>>,
        engine: Arc<Engine>,
        graph: Arc<Mutex<AudioGraph>>,
        _assets: &Arc<AppAssets>,
        cx: &mut Context<Self>,
    ) -> Self {
        let overlays = Arc::new(Mutex::new(Vec::new()));
        let overlay_ids = Arc::new(Mutex::new(Vec::new()));

        let entity = Self {
            shared: shared.clone(),
            engine,
            graph,
            overlay: overlays,
            overlay_ids,
            search_focus: cx.focus_handle(),
            editor_focus: cx.focus_handle(),
            editor_search_focus: cx.focus_handle(),
            browse_focus: cx.focus_handle(),
            hovered_card: None,
            vol_dragging: false,
            vol_track: Arc::new(Mutex::new(None)),
            progress_dragging: false,
            progress_track: Arc::new(Mutex::new(None)),
            text_drag: None,
            last_ver: 1,
            last_overlay_active: false,
            close_hooked: false,
        };

        // Um overlay fullscreen por display: o compositor fixa cada surface
        // layer-shell num output — overlay único ficava preso no monitor
        // errado sem receber o mouse do outro. 1x1 (invisível) até ativar.
        // `ensure_overlays` (no tick) completa monitores que aparecem depois
        // (a enumeração Wayland pode chegar incompleta na largada).
        entity.ensure_overlays(cx);

        entity.spawn_background_tasks(cx);
        entity.spawn_poller(cx);
        entity.spawn_fs_watcher();
        entity
    }

    /// Garante um overlay por display (idempotente e silencioso quando
    /// estável): cria só para displays sem janela e fecha janelas obsoletas
    /// (ex. a transitória `None` da largada, quando a enumeração Wayland
    /// ainda estava vazia).
    fn ensure_overlays(&self, cx: &mut Context<Self>) {
        use std::collections::HashSet;
        let displays = cx.displays();
        // Sem displays (enumeração ainda vazia): tenta de novo no próximo
        // tick em vez de criar janela fantasma `None`.
        if displays.is_empty() {
            return;
        }
        let want: HashSet<Option<open_gpui::DisplayId>> =
            displays.iter().map(|d| Some(d.id())).collect();
        // Fecha obsoletas fora do lock.
        let stale: Vec<WindowHandle<OverlayEntity>> = {
            let guard = self.overlay.lock().unwrap();
            let have: HashSet<Option<open_gpui::DisplayId>> =
                guard.iter().map(|(_, id)| *id).collect();
            if have == want {
                return;
            }
            guard
                .iter()
                .filter(|(_, id)| !want.contains(id))
                .map(|(h, _)| h.clone())
                .collect()
        };
        for handle in stale {
            log::info!("overlay: fechando janela obsoleta");
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            self.overlay.lock().unwrap().retain(|(h, _)| h != &handle);
        }
        let missing: Vec<Option<open_gpui::DisplayId>> = {
            let guard = self.overlay.lock().unwrap();
            let have: HashSet<Option<open_gpui::DisplayId>> =
                guard.iter().map(|(_, id)| *id).collect();
            want.difference(&have).copied().collect()
        };
        if missing.is_empty() {
            return;
        }
        let primary_id = cx
            .primary_display()
            .map(|d| d.id())
            .or_else(|| displays.first().map(|d| d.id()));
        log::info!("overlay: {} display(s)", displays.len());
        for d in &displays {
            log::info!("overlay: display {:?} bounds {:?}", d.id(), d.bounds());
        }
        for target in missing {
            let is_fallback = target == primary_id || (primary_id.is_none() && target.is_none());
            if Self::open_overlay(
                self.overlay.clone(),
                self.overlay_ids.clone(),
                self.shared.clone(),
                target,
                is_fallback,
                cx,
            )
            .is_err()
            {
                break;
            }
        }
    }

    /// Abre uma janela de overlay para `target` e registra em `overlays`.
    /// Retorna `Err` se nem LayerShell nem o fallback PopUp abriram.
    fn open_overlay(
        overlays: Arc<Mutex<Vec<(WindowHandle<OverlayEntity>, Option<open_gpui::DisplayId>)>>>,
        overlay_ids: Arc<Mutex<Vec<open_gpui::EntityId>>>,
        shared: Arc<Mutex<Shared>>,
        target: Option<open_gpui::DisplayId>,
        is_fallback: bool,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let shared_layer = shared.clone();
        let shared_popup = shared.clone();
        let ids_layer = overlay_ids.clone();
        let ids_popup = overlay_ids.clone();
        let layer_opts = default_layer_shell_options();
        cx.open_window(
            overlay_options(WindowKind::LayerShell(layer_opts), target),
            move |_window, cx| {
                let e = cx.new(|cx| {
                    OverlayEntity::new(
                        shared_layer.clone(),
                        ids_layer.clone(),
                        target,
                        is_fallback,
                        cx,
                    )
                });
                let mut ids = ids_layer.lock().unwrap();
                if !ids.contains(&e.entity_id()) {
                    ids.push(e.entity_id());
                }
                e
            },
        )
        .or_else(|_| {
            // Compositor sem LayerShell: cai para uma janela xdg normal (PopUp).
            cx.open_window(
                overlay_options(WindowKind::PopUp, target),
                move |_window, cx| {
                    let e = cx.new(|cx| {
                        OverlayEntity::new(
                            shared_popup.clone(),
                            ids_popup.clone(),
                            target,
                            is_fallback,
                            cx,
                        )
                    });
                    let mut ids = ids_popup.lock().unwrap();
                    if !ids.contains(&e.entity_id()) {
                        ids.push(e.entity_id());
                    }
                    e
                },
            )
        })
        .map(|handle| {
            log::info!("overlay: janela criada para display {target:?}");
            overlays.lock().unwrap().push((handle, target))
        })
        .map_err(|err| {
            log::warn!("não foi possível criar a janela de overlay ({target:?}): {err}");
            anyhow::anyhow!("{err}")
        })
    }

    /// Observa a pasta de sons e espelha mudanças externas na UI.
    /// Uma vez por processo (reabrir a janela não duplica o watcher).
    fn spawn_fs_watcher(&self) {
        static WATCHER_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        let shared = self.shared.clone();
        let engine = self.engine.clone();
        WATCHER_ONCE.get_or_init(|| {
            std::thread::Builder::new()
                .name("klipp-watch".into())
                .spawn(move || {
                    if let Err(err) = backend::sounds::watch_loop(shared, engine) {
                        log::warn!("fs watcher encerrou: {err:?}");
                    }
                })
                .ok();
        });
    }

    // -- services (backend) -------------------------------------------------

    fn spawn_background_tasks(&self, cx: &mut Context<Self>) {
        // Inicializa o grafo de áudio (virtual mic) em background.
        // (Roda a cada janela: re-sincroniza o mic após reabrir via tray.)
        let graph_bg = self.graph.clone();
        let shared_g = self.shared.clone();
        cx.background_executor()
            .spawn(async move {
                let (preferred, mic_pass, hear) = {
                    let s = shared_g.lock().unwrap();
                    (s.mic_source.clone(), s.mic_pass, s.hear_clips)
                };
                let result = graph_bg.lock().unwrap().init(&preferred, mic_pass, hear);
                let mic_label = graph_bg.lock().unwrap().mic_source().to_string();
                let mut s = shared_g.lock().unwrap();
                s.mic_source = mic_label.clone();
                s.mic_sources = backend::list_sources();
                // Se a fonte salva desplugou, volta ao auto-detect resolvido.
                if !preferred.is_empty() && !s.mic_sources.iter().any(|(n, _)| n == &preferred) {
                    s.mic_source = mic_label.clone();
                }
                s.mic = match result {
                    Ok(()) => {
                        if mic_label.is_empty() {
                            "ok · mic virtual".into()
                        } else {
                            format!("ok · mic virtual ← {mic_label}")
                        }
                    }
                    Err(err) => format!("erro: {err}"),
                };
                s.sounds = backend::list_sounds();
                s.bump();
                drop(s);
                backend::probe_missing_durations(shared_g);
            })
            .detach();

        // Escuta o portal de atalhos globais (uma vez por processo: o
        // `Shared` é único, um segundo listener duplicaria os disparos).
        // Mesmo padrão para o tray icon e o fs watcher.
        static SHORTCUTS_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        static TRAY_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        let shared_h = self.shared.clone();
        let preferred = self.shared.lock().unwrap().shortcut.clone();
        SHORTCUTS_ONCE.get_or_init(|| {
            cx.background_executor()
                .spawn(async move {
                    if let Err(err) = backend::shortcuts::run(shared_h, preferred).await {
                        log::warn!("atalho global indisponível: {err}");
                    }
                })
                .detach();
        });
        let shared_t = self.shared.clone();
        TRAY_ONCE.get_or_init(|| {
            cx.background_executor()
                .spawn(async move {
                    backend::tray::run(shared_t).await;
                })
                .detach();
        });
    }

    fn spawn_poller(&self, cx: &mut Context<Self>) {
        // Aplica pedidos (play/confirm/resize do overlay) e re-renderiza.
        // ~15fps para as animações (caret, EQ) sem gastar CPU à toa.
        let shared_auto = self.shared.clone();
        cx.spawn(
            |this: open_gpui::WeakEntity<Self>, cx: &mut open_gpui::AsyncApp| {
                let cx = cx.clone();
                async move {
                    maybe_autoplay(&shared_auto, &cx, &this).await;
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(66))
                            .await;
                        // Janela fechada (modo tray): encerra o poller em vez
                        // de girar em falso sobre a entidade morta.
                        let alive = cx.update(|app| {
                            this.update(app, |this, cx| this.on_tick(cx)).is_ok()
                        });
                        if !alive {
                            break;
                        }
                    }
                }
            },
        )
        .detach();
    }

    fn on_tick(&mut self, cx: &mut Context<Self>) {
        // Monitores que aparecem depois da largada ganham overlay aqui.
        self.ensure_overlays(cx);
        // Âncora exata (thread) e hover: sincroniza as janelas de overlay.
        self.sync_overlay_windows(cx);
        // Pedido de play fora do guard: `Engine::play` publica a época no
        // `Shared` de forma síncrona (travar aqui = deadlock na UI thread).
        let play_now: Option<Sound> = {
            let mut s = self.shared.lock().unwrap();
            s.play_request
                .take()
                .and_then(|name| s.sounds.iter().find(|sound| sound.name == name).cloned())
        };
        if let Some(sound) = play_now {
            self.engine
                .play(sound.path.clone(), sound.name.clone(), self.shared.clone());
        }
        let mut s = self.shared.lock().unwrap();

        if s.stop_request {
            s.stop_request = false;
            self.engine.stop();
        }

        if s.confirm_request {
            s.confirm_request = false;
            if s.overlay_active && !s.overlay_fading {
                if let Some(idx) = s.pie_hovered {
                    if let Some(sound) = s.favorite_sounds().get(idx) {
                        s.play_request = Some(sound.name.clone());
                    }
                } else if s.center_hovered && s.playing.is_some() {
                    // Soltou o atalho sobre o botão central: para o áudio.
                    s.stop_request = true;
                }
                // Fecha com fade-out rápido (~110ms): mantém montado, o
                // trecho abaixo desmonta (ver `overlay_fading`).
                s.overlay_fading = true;
                s.overlay_fade_start = super::format::now_ms();
                s.bump();
            }
        }
        // Fim do fade-out: desmonta o overlay (janelas voltam a 1x1).
        if s.overlay_fading && super::format::now_ms().saturating_sub(s.overlay_fade_start) >= 110 {
            s.overlay_active = false;
            s.overlay_fading = false;
            s.overlay_anchor = None;
            s.anchor_needs_confirm = false;
            s.pie_hovered = None;
            s.center_hovered = false;
            s.bump();
        }

        let overlay_active = s.overlay_active;
        let version = s.version;
        // Arrastar arquivos externos: reflete no overlay via poll (~15fps).
        let ext_drag = cx.active_drag_value::<open_gpui::ExternalPaths>().is_some();
        if ext_drag != s.drop_active {
            s.drop_active = ext_drag;
            s.bump();
        }
        // Repinta em loop enquanto algo anima (caret, EQ, drawer/modal).
        // Switches usam `with_animation` (frames próprios, sem poll).
        let animated = s.search_focused
            || s.browse_focused
            || s.editor_open
            || s.playing.is_some()
            || s.settings_closing
            || s.browse_closing
            || s.about_closing
            || s.editor_closing;
        // Fim da animação de saída do drawer: desmonta.
        if s.settings_closing
            && super::format::now_ms().saturating_sub(s.settings_anim_start) >= 240
        {
            s.settings_open = false;
            s.settings_closing = false;
            s.bump();
        }
        if s.browse_closing && super::format::now_ms().saturating_sub(s.browse_anim_start) >= 240 {
            s.browse_open = false;
            s.browse_closing = false;
            s.browse_focused = false;
            s.bump();
        }
        // Fim do fade de saída dos dialogs: desmonta.
        if s.about_closing && super::format::now_ms().saturating_sub(s.about_anim_start) >= 200 {
            s.about_open = false;
            s.about_closing = false;
            s.bump();
        }
        if s.editor_closing && super::format::now_ms().saturating_sub(s.editor_anim_start) >= 200 {
            s.editor_open = false;
            s.editor_closing = false;
            s.editor_name_focused = false;
            s.editor_search_focused = false;
            s.bump();
        }
        // Preview online terminou (ou parou): volta o ícone da linha.
        // Com 1s de carência após o início: cobre o vão entre
        // `loading=false` e o `playing=Some` da thread de playback.
        if s.browse_previewing.is_some()
            && !s.browse_preview_loading
            && s.playing.is_none()
            && super::format::now_ms().saturating_sub(s.browse_preview_started_ms) >= 1000
        {
            s.browse_previewing = None;
            s.bump();
        }
        drop(s);

        if overlay_active != self.last_overlay_active {
            self.last_overlay_active = overlay_active;
            self.resize_overlay(cx, overlay_active);
        }

        if version != self.last_ver {
            self.last_ver = version;
            cx.notify();
        } else if animated {
            cx.notify();
        }
    }

    /// Sincroniza as janelas de overlay enquanto ativo: confirma a âncora
    /// via hover quando pendente e repinta todas (a âncora exata chega por
    /// thread dedicada, fora do ciclo de repaint das janelas).
    fn sync_overlay_windows(&self, cx: &mut Context<Self>) {
        if !self.shared.lock().unwrap().overlay_active {
            return;
        }
        let needs = self.shared.lock().unwrap().anchor_needs_confirm;
        let origins: std::collections::HashMap<Option<open_gpui::DisplayId>, (f32, f32)> = cx
            .displays()
            .iter()
            .map(|d| {
                let o = d.bounds().origin;
                (Some(d.id()), (f32::from(o.x), f32::from(o.y)))
            })
            .collect();
        let shared = self.shared.clone();
        for (handle, target) in self.overlay.lock().unwrap().iter() {
            let origin = origins.get(target).copied();
            let _ = handle.update(cx, |_, window, cx| {
                if needs {
                    if let Some((ox, oy)) = origin {
                        if window.is_mouse_in_window() {
                            let p = window.mouse_position();
                            let pos = (f32::from(p.x) + ox, f32::from(p.y) + oy);
                            let mut s = shared.lock().unwrap();
                            s.anchor_needs_confirm = false;
                            s.overlay_anchor = Some(pos);
                            s.bump();
                            log::info!("overlay âncora (hover): ({:.0}, {:.0})", pos.0, pos.1);
                        }
                    }
                }
                cx.notify();
            });
        }
    }

    fn resize_overlay(&self, cx: &mut Context<Self>, active: bool) {
        // Cada overlay cobre o seu display (lookup por display_id, sem
        // depender de ordem); inativo volta a 1x1.
        let bounds_by_display: std::collections::HashMap<
            Option<open_gpui::DisplayId>,
            open_gpui::Bounds<open_gpui::Pixels>,
        > = if active {
            cx.displays()
                .iter()
                .map(|d| (Some(d.id()), d.bounds()))
                .collect()
        } else {
            Default::default()
        };
        for (handle, target) in self.overlay.lock().unwrap().iter() {
            let bounds = bounds_by_display.get(target).copied();
            let size = bounds
                .map(|b| b.size)
                .unwrap_or(open_gpui::size(px(1.0), px(1.0)));
            // Origem real do display (window.bounds() de layer-shell mente:
            // sempre 1x1@(0,0)).
            let origin = bounds.map(|b| b.origin);
            let shared = self.shared.clone();
            let _ = handle.update(cx, |this, window, cx| {
                window.resize(size);
                if active {
                    log::info!("overlay janela {target:?}: display {bounds:?}");
                    // Sem activate_window: layer Overlay já fica no topo por
                    // protocolo, e pedir ativação só gera erro do backend
                    // ("activation token received with no pending activation").
                    if window.is_mouse_in_window() {
                        let mut s = shared.lock().unwrap();
                        if s.overlay_anchor.is_none() {
                            let p = window.mouse_position();
                            let anchor = match origin {
                                Some(o) => (
                                    f32::from(p.x) + f32::from(o.x),
                                    f32::from(p.y) + f32::from(o.y),
                                ),
                                None => (f32::from(p.x), f32::from(p.y)),
                            };
                            log::info!("overlay âncora (pré): ({:.0}, {:.0})", anchor.0, anchor.1);
                            s.overlay_anchor = Some(anchor);
                            s.anchor_needs_confirm = false;
                            s.bump();
                        }
                    }
                }
                this.set_active(active, cx);
            });
        }
        if active {
            log::info!(
                "overlay ativado: {} janela(s), {} display(s)",
                self.overlay.lock().unwrap().len(),
                bounds_by_display.len()
            );
        }
    }

    // -- ações de UI (usadas pelos componentes) -------------------------------

    pub(crate) fn play_sound(&self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        {
            let mut s = self.shared.lock().unwrap();
            s.play_request = Some(name);
            s.search_focused = false;
        }
        window.blur();
        cx.notify();
    }

    pub(crate) fn rescan(&self) {
        let mut s = self.shared.lock().unwrap();
        s.sounds = backend::list_sounds();
        s.bump();
        drop(s);
        backend::probe_missing_durations(self.shared.clone());
    }

    /// Persiste prefs (density, sort, theme, lang, volume, toggles, mic).
    fn persist_prefs(&self) {
        let s = self.shared.lock().unwrap();
        crate::core::settings::save(&crate::core::settings::Settings {
            shortcut: s.shortcut.clone(),
            density: s.density.clone(),
            sort: s.sort.clone(),
            theme: s.theme.clone(),
            language: s.lang.clone(),
            mic_passthrough: s.mic_pass,
            hear_clips: s.hear_clips,
            run_in_background: s.run_in_background,
            show_hints: s.show_hint,
            volume: s.volume,
            mic_source: s.mic_source.clone(),
        });
    }

    pub(crate) fn dismiss_hint(&self, cx: &mut Context<Self>) {
        self.shared.lock().unwrap().show_hint = false;
        self.shared.lock().unwrap().bump();
        self.persist_prefs();
        cx.notify();
    }

    pub(crate) fn toggle_hints(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.show_hint = !s.show_hint;
        s.bump();
        drop(s);
        self.persist_prefs();
        cx.notify();
    }

    pub(crate) fn toggle_favorite(&self, name: String, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.unfavorited.contains(&name) {
            s.unfavorited.remove(&name);
        } else {
            s.unfavorited.insert(name);
        }
        s.bump();
        cx.notify();
    }

    pub(crate) fn toggle_only_favorites(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.only_favorites = !s.only_favorites;
        s.bump();
        cx.notify();
    }

    pub(crate) fn delete_sound(&self, name: String, cx: &mut Context<Self>) {
        let playing = self.shared.lock().unwrap().playing.clone();
        if playing.as_ref() == Some(&name) {
            self.engine.stop();
        }
        if backend::sounds::delete_sound(&name) {
            log::info!("som removido: {name}");
        }
        let mut s = self.shared.lock().unwrap();
        s.unfavorited.remove(&name);
        s.sounds = backend::list_sounds();
        s.bump();
        cx.notify();
    }

    pub(crate) fn focus_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.search_focus, cx);
        let mut s = self.shared.lock().unwrap();
        focus_field(&mut s, TextField::Search);
        // Sem mexer na seleção: o mouse_down que precede o clique já
        // recolheu (e o duplo-clique já selecionou).
        s.bump();
        cx.notify();
    }

    /// Mouse num input custom: clique simples foca e recolhe a seleção
    /// (iniciando o tracking de arrasto); duplo-clique seleciona tudo.
    /// O caller dá `stop_propagation` para não cair no blur global.
    pub(crate) fn input_mouse_down(
        &mut self,
        field: TextField,
        x: f32,
        clicks: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match field {
            TextField::Search => window.focus(&self.search_focus, cx),
            TextField::Browse => window.focus(&self.browse_focus, cx),
            TextField::EditorName => window.focus(&self.editor_focus, cx),
            TextField::EditorSearch => window.focus(&self.editor_search_focus, cx),
        }
        let mut s = self.shared.lock().unwrap();
        focus_field(&mut s, field);
        let has_text = match field {
            TextField::Search => !s.search.is_empty(),
            TextField::Browse => !s.browse_query.is_empty(),
            TextField::EditorName => !s.editor_display.is_empty(),
            TextField::EditorSearch => !s.editor_emoji_query.is_empty(),
        };
        if clicks >= 2 {
            self.text_drag = None;
            if has_text {
                match field {
                    TextField::Search => s.search_selected = true,
                    TextField::Browse => s.browse_selected = true,
                    TextField::EditorName => s.editor_name_selected = true,
                    TextField::EditorSearch => s.editor_search_selected = true,
                }
            }
        } else {
            match field {
                TextField::Search => s.search_selected = false,
                TextField::Browse => s.browse_selected = false,
                TextField::EditorName => s.editor_name_selected = false,
                TextField::EditorSearch => s.editor_search_selected = false,
            }
            self.text_drag = Some((field, x));
        }
        s.bump();
        cx.notify();
    }

    /// Soltar o botão no input: arrasto além de 4px seleciona tudo.
    pub(crate) fn input_mouse_up(&mut self, field: TextField, x: f32, cx: &mut Context<Self>) {
        let dragged = match self.text_drag.take() {
            Some((f, x0)) => f == field && (x - x0).abs() > 4.0,
            None => false,
        };
        if !dragged {
            return;
        }
        let mut s = self.shared.lock().unwrap();
        let has_text = match field {
            TextField::Search => !s.search.is_empty(),
            TextField::Browse => !s.browse_query.is_empty(),
            TextField::EditorName => !s.editor_display.is_empty(),
            TextField::EditorSearch => !s.editor_emoji_query.is_empty(),
        };
        if has_text {
            match field {
                TextField::Search => s.search_selected = true,
                TextField::Browse => s.browse_selected = true,
                TextField::EditorName => s.editor_name_selected = true,
                TextField::EditorSearch => s.editor_search_selected = true,
            }
            s.bump();
            cx.notify();
        }
    }

    pub(crate) fn on_search_key(
        &self,
        event: &open_gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ks = &event.keystroke;
        // Ctrl+A / Cmd+A seleciona a busca toda.
        if (ks.modifiers.control || ks.modifiers.platform)
            && !ks.modifiers.alt
            && ks.key.eq_ignore_ascii_case("a")
        {
            let mut s = self.shared.lock().unwrap();
            if !s.search.is_empty() {
                s.search_selected = true;
                s.bump();
                cx.notify();
            }
            return;
        }
        let mut s = self.shared.lock().unwrap();
        match ks.key.as_str() {
            "backspace" => {
                if s.search_selected {
                    s.search.clear();
                    s.search_selected = false;
                } else {
                    s.search.pop();
                }
            }
            "escape" => {
                if s.search_selected {
                    // Primeiro Esc só solta a seleção.
                    s.search_selected = false;
                } else {
                    s.search.clear();
                    s.search_focused = false;
                    window.blur();
                }
            }
            "enter" => {
                s.search_focused = false;
                window.blur();
            }
            _ => {
                if !ks.modifiers.control && !ks.modifiers.alt && !ks.modifiers.platform {
                    if let Some(ch) = ks.key_char.clone() {
                        if ch.chars().count() == 1 {
                            if s.search_selected {
                                s.search = ch;
                                s.search_selected = false;
                            } else {
                                s.search.push_str(&ch);
                            }
                        } else if s.search_selected {
                            // Setas e cia: soltam a seleção sem mexer no texto.
                            s.search_selected = false;
                        }
                    } else if s.search_selected {
                        s.search_selected = false;
                    }
                }
            }
        }
        s.bump();
        cx.notify();
    }

    pub(crate) fn set_hovered(&mut self, idx: Option<usize>, cx: &mut Context<Self>) {
        if self.hovered_card != idx {
            self.hovered_card = idx;
            cx.notify();
        }
    }

    pub(crate) fn blur_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.search_focused {
            s.search_focused = false;
            s.search_selected = false;
            s.bump();
            window.blur();
            cx.notify();
        }
    }

    pub(crate) fn clear_search(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.search.clear();
        s.search_selected = false;
        s.bump();
        cx.notify();
    }

    pub(crate) fn set_volume(&self, volume: f32, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.volume = volume.clamp(0.0, 1.0);
        // Mexer no slider desmuta (só chamado pelo slider).
        s.muted = false;
        s.bump();
        cx.notify();
    }

    pub(crate) fn toggle_mute(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.muted {
            s.muted = false;
        } else {
            if s.volume > 0.01 {
                s.pre_mute_volume = s.volume;
            }
            s.muted = true;
        }
        s.bump();
        cx.notify();
    }

    /// Fração 0–1 do clique na trilha, via retângulo medido no layout.
    pub(crate) fn vol_fraction_at(&self, x: f32) -> Option<f32> {
        let bounds = self.vol_track.lock().unwrap().clone()?;
        vol_fraction_in(bounds, x)
    }
    pub(crate) fn on_vol_down(&mut self, x: f32, cx: &mut Context<Self>) {
        // Pulo imediato para onde clicou; o arrasto continua daí.
        if let Some(fraction) = self.vol_fraction_at(x) {
            self.set_volume(fraction, cx);
        }
        self.vol_dragging = true;
    }

    pub(crate) fn on_vol_move(&mut self, x: f32, cx: &mut Context<Self>) {
        if self.vol_dragging {
            if let Some(fraction) = self.vol_fraction_at(x) {
                self.set_volume(fraction, cx);
            }
        }
    }

    /// Seek absoluto em segundos: a thread de playback consome via
    /// `seek_request`; o offset é atualizado na hora para a UI responder
    /// sem esperar o próximo chunk de áudio.
    pub(crate) fn seek_to(&self, secs: f32, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        let Some(duration) = s.play_duration_secs.filter(|d| *d > 0.0) else {
            return;
        };
        if s.playing.is_none() {
            return;
        }
        let clamped = secs.clamp(0.0, duration);
        s.seek_request = Some(clamped);
        // Feedback otimista: a UI mostra o alvo já neste frame.
        s.play_offset_secs = clamped;
        s.play_start_ms = super::format::now_ms();
        s.bump();
        cx.notify();
    }

    /// Fração 0–1 do clique na trilha de progresso.
    pub(crate) fn progress_fraction_at(&self, x: f32) -> Option<f32> {
        let bounds = self.progress_track.lock().unwrap().clone()?;
        vol_fraction_in(bounds, x)
    }

    pub(crate) fn on_progress_down(&mut self, x: f32, cx: &mut Context<Self>) {
        if let Some(fraction) = self.progress_fraction_at(x) {
            let duration = self
                .shared
                .lock()
                .unwrap()
                .play_duration_secs
                .filter(|d| *d > 0.0);
            if let Some(d) = duration {
                self.seek_to(fraction * d, cx);
            }
        }
        self.progress_dragging = true;
    }

    pub(crate) fn on_progress_move(&mut self, x: f32, cx: &mut Context<Self>) {
        if self.progress_dragging {
            if let Some(fraction) = self.progress_fraction_at(x) {
                let duration = self
                    .shared
                    .lock()
                    .unwrap()
                    .play_duration_secs
                    .filter(|d| *d > 0.0);
                if let Some(d) = duration {
                    self.seek_to(fraction * d, cx);
                }
            }
        }
    }

    pub(crate) fn end_progress_drag(&mut self) {
        self.progress_dragging = false;
    }

    pub(crate) fn toggle_play(&self, cx: &mut Context<Self>) {
        let (playing, paused) = {
            let s = self.shared.lock().unwrap();
            (s.playing.is_some(), s.play_paused)
        };
        match (playing, paused) {
            // Tocando: pausa (mantém o índice). Pausado: retoma.
            (true, false) => self.engine.pause(&self.shared),
            (true, true) => self.engine.resume(&self.shared),
            // Parado: toca o primeiro som da lista.
            (false, _) => {
                let first = self.shared.lock().unwrap().sounds.first().cloned();
                if let Some(sound) = first {
                    let shared = self.shared.clone();
                    self.engine.play(sound.path, sound.name, shared);
                }
            }
        }
        cx.notify();
    }

    // -- drawer de settings --------------------------------------------------------

    pub(crate) fn toggle_settings(&self, cx: &mut Context<Self>) {
        let open = self.shared.lock().unwrap().settings_open;
        if open {
            self.close_settings(cx);
        } else {
            let mut s = self.shared.lock().unwrap();
            s.settings_open = true;
            s.settings_closing = false;
            s.bump();
            cx.notify();
        }
    }

    /// Botão do atalho: reabre o popup de escolha do portal.
    pub(crate) fn rebind_shortcut(&self, cx: &mut Context<Self>) {
        if self.shared.lock().unwrap().shortcut_rebinding {
            return;
        }
        // Feedback imediato (a task de portal pode demorar a acordar).
        {
            let mut s = self.shared.lock().unwrap();
            s.shortcut_rebinding = true;
            s.bump();
        }
        cx.notify();
        let shared = self.shared.clone();
        cx.background_executor()
            .spawn(async move {
                if let Err(err) = backend::shortcuts::rebind(shared).await {
                    log::warn!("rebind de atalho: {err}");
                }
            })
            .detach();
    }

    pub(crate) fn close_settings(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        if s.settings_open && !s.settings_closing {
            // Mantém montado: a animação reversa toca, o tick desmonta.
            s.settings_closing = true;
            s.settings_anim_start = super::format::now_ms();
            s.bump();
            cx.notify();
        }
    }

    pub(crate) fn set_mic_pass(&self, on: bool, cx: &mut Context<Self>) {
        let result = self.graph.lock().unwrap().set_mic_passthrough(on);
        let mut s = self.shared.lock().unwrap();
        match result {
            Ok(()) => s.mic_pass = on,
            Err(err) => s.last_error = Some(format!("mic pass-through: {err}")),
        }
        s.bump();
        drop(s);
        self.persist_prefs();
        cx.notify();
    }

    pub(crate) fn set_hear_clips(&self, on: bool, cx: &mut Context<Self>) {
        let result = self.graph.lock().unwrap().set_hear_clips(on);
        let mut s = self.shared.lock().unwrap();
        match result {
            Ok(()) => s.hear_clips = on,
            Err(err) => s.last_error = Some(format!("monitoring: {err}")),
        }
        s.bump();
        drop(s);
        self.persist_prefs();
        cx.notify();
    }

    /// Liga/desliga o "continuar no tray ao fechar". Lido ao vivo nos
    /// caminhos de fechar, então vale na hora (sem reiniciar).
    pub(crate) fn set_run_in_background(&self, on: bool, cx: &mut Context<Self>) {
        self.shared.lock().unwrap().run_in_background = on;
        self.shared.lock().unwrap().bump();
        self.persist_prefs();
        cx.notify();
    }

    /// X da titlebar: fecha só a janela (o tray reabre) ou encerra tudo,
    /// conforme a configuração.
    pub(crate) fn request_close(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .shared
            .lock()
            .map(|s| s.run_in_background)
            .unwrap_or(true)
        {
            window.remove_window();
        } else {
            cx.quit();
        }
    }

    pub(crate) fn toggle_mic_open(&self, cx: &mut Context<Self>) {
        // Atualiza a lista a cada abertura (dispositivo pode ter plugado).
        if !self.shared.lock().unwrap().mic_open {
            let sources = backend::list_sources();
            self.shared.lock().unwrap().mic_sources = sources;
        }
        let mut s = self.shared.lock().unwrap();
        s.mic_open = !s.mic_open;
        s.bump();
        cx.notify();
    }

    pub(crate) fn select_mic_source(&self, name: String, cx: &mut Context<Self>) {
        let pass = self.shared.lock().unwrap().mic_pass;
        let result = self.graph.lock().unwrap().set_mic_source(&name, pass);
        let mut s = self.shared.lock().unwrap();
        match result {
            Ok(()) => {
                s.mic_source = name;
                s.mic = format!("ok · mic virtual ← {}", s.mic_source.clone());
            }
            Err(err) => {
                let lang = s.lang.clone();
                let msg = err.to_string();
                s.last_error = Some(t_fmt(&lang, "err.mic", &[("msg", &msg)]));
            }
        }
        s.mic_open = false;
        s.bump();
        drop(s);
        self.persist_prefs();
        cx.notify();
    }

    pub(crate) fn set_density(&self, density: &str, cx: &mut Context<Self>) {
        self.shared.lock().unwrap().density = density.to_string();
        self.shared.lock().unwrap().bump();
        self.persist_prefs();
        cx.notify();
    }

    pub(crate) fn toggle_sort_open(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.sort_open = !s.sort_open;
        s.bump();
        cx.notify();
    }

    pub(crate) fn toggle_lang_open(&self, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.lang_open = !s.lang_open;
        s.bump();
        cx.notify();
    }

    pub(crate) fn set_language(&self, lang: &str, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.lang = lang.to_string();
        s.lang_open = false;
        s.bump();
        drop(s);
        self.persist_prefs();
        cx.notify();
    }

    pub(crate) fn set_sort(&self, sort: &str, cx: &mut Context<Self>) {
        let mut s = self.shared.lock().unwrap();
        s.sort = sort.to_string();
        s.sort_open = false;
        s.bump();
        drop(s);
        self.persist_prefs();
        cx.notify();
    }

    pub(crate) fn set_theme(&self, theme: &str, cx: &mut Context<Self>) {
        self.shared.lock().unwrap().theme = theme.to_string();
        self.shared.lock().unwrap().bump();
        // Transições capturam cores: sem reset, elementos com transição
        // (cards, inputs) congelam a paleta antiga ao trocar de tema.
        gpui_animation::reset_all_transitions();
        self.persist_prefs();
        cx.notify();
    }

    /// Fim do arrasto/clique do volume: persiste.
    pub(crate) fn end_vol_drag(&mut self) {
        self.vol_dragging = false;
        self.persist_prefs();
    }

    /// Abre a pasta de sons no gerenciador de arquivos e atualiza a lista.
    pub(crate) fn open_sounds_folder(&self) {
        let dir = backend::sounds::sounds_dir();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
        self.rescan();
    }

    /// Botão "+": seletor de arquivos do portal (não bloqueia a UI).
    pub(crate) fn pick_files(&self, cx: &mut Context<Self>) {
        let shared = self.shared.clone();
        cx.background_executor()
            .spawn(async move {
                if let Err(err) = backend::pick::pick_and_import(shared).await {
                    log::warn!("seletor de arquivos: {err}");
                }
            })
            .detach();
    }

    /// Drop de arquivos do SO sobre a janela.
    pub(crate) fn import_dropped(&self, paths: &[std::path::PathBuf], cx: &mut Context<Self>) {
        let n = backend::import_paths(paths);
        let mut s = self.shared.lock().unwrap();
        if n == 0 {
            s.last_error = Some(t(&s.lang.clone(), "err.import_none"));
            s.bump();
        } else {
            log::info!("{n} sons importados via arrastar-e-soltar");
        }
        drop(s);
        if n > 0 {
            self.rescan();
        }
        cx.notify();
    }
}

async fn maybe_autoplay(
    shared: &Arc<Mutex<Shared>>,
    cx: &open_gpui::AsyncApp,
    this: &open_gpui::WeakEntity<MainWindow>,
) {
    let Ok(path) = std::env::var("KLIPP_AUTOPLAY") else {
        return;
    };
    std::thread::sleep(std::time::Duration::from_millis(1500));
    let autoplay = shared
        .lock()
        .unwrap()
        .sounds
        .iter()
        .find(|s| path == s.name || path.ends_with(&s.name))
        .cloned()
        .map(|s| (s.name, s.path));
    if let Some((name, path)) = autoplay {
        log::info!("KLIPP_AUTOPLAY: tocando {name}");
        let _ = cx.update(|app| {
            let _ = this.update(app, |this, _cx| {
                let shared = this.shared.clone();
                this.engine.play(path, name, shared);
            });
            log::info!("play enfileirado");
        });
    }
}

fn overlay_options(kind: WindowKind, display_id: Option<open_gpui::DisplayId>) -> WindowOptions {
    WindowOptions {
        kind,
        window_bounds: Some(WindowBounds::Windowed(open_gpui::Bounds {
            origin: open_gpui::point(px(0.0), px(0.0)),
            size: size(px(1.0), px(1.0)),
        })),
        focus: false,
        show: true,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        display_id,
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some("io.github.ErnestoMuniz.Klipp".into()),
        titlebar: None,
        ..Default::default()
    }
}

fn default_layer_shell_options() -> LayerShellOptions {
    LayerShellOptions {
        namespace: "io.github.ErnestoMuniz.Klipp".into(),
        layer: Layer::Overlay,
        // Fullscreen: ancora nas 4 bordas para esticar na tela inteira.
        // Com `Anchor::empty()` a janela ficava flutuante/centrada e o
        // resize para o tamanho do display não cobria a tela — o overlay
        // parecia "não aparecer".
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        exclusive_zone: None,
        exclusive_edge: None,
        margin: None,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
    }
}

// -- render ------------------------------------------------------------------

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Intercepta o fechar do WM (ex. Alt+F4): com "rodar em fundo"
        // ligado a janela fecha e o app continua no tray; desligado,
        // encerra (o X da titlebar passa pelo mesmo `request_close`).
        // Sem isso, fechar pelo WM deixava o processo zumbi (os overlays
        // por display impedem o auto-quit do GPUI).
        if !self.close_hooked {
            self.close_hooked = true;
            let shared = self.shared.clone();
            window.on_window_should_close(cx, move |_window, cx| {
                if !shared.lock().map(|s| s.run_in_background).unwrap_or(true) {
                    cx.quit();
                }
                true
            });
        }
        let (
            shortcut,
            last_error,
            sounds,
            playing,
            show_hint,
            query,
            search_focused,
            volume,
            muted,
            only_fav,
            unfavorited,
            compact,
            settings_open,
            sort,
            sort_open,
            theme_pref,
            drop_active,
            lang,
            about_open,
            browse_open,
            editor_open,
            playback,
            play_peaks,
            play_paused,
        ) = {
            let shared = self.shared.lock().unwrap();
            let now = super::format::now_ms();
            let playback = shared.playback_pos(now);
            (
                shared.shortcut.clone(),
                shared.last_error.clone(),
                shared.sounds.clone(),
                shared.playing.clone(),
                shared.show_hint,
                shared.search.clone(),
                shared.search_focused,
                shared.volume,
                shared.muted,
                shared.only_favorites,
                shared.unfavorited.clone(),
                shared.density == "compact",
                shared.settings_open,
                shared.sort.clone(),
                shared.sort_open,
                shared.theme.clone(),
                shared.drop_active,
                shared.lang.clone(),
                shared.about_open,
                shared.browse_open,
                shared.editor_open,
                playback,
                shared.play_peaks.clone(),
                shared.play_paused,
            )
        };
        // Tema global (todas as cores de `theme::` passam a ler a paleta ativa).
        // "system" = escuro por enquanto (sem detecção do SO).
        super::theme::set_light(theme_pref == "light");

        let q = query.to_lowercase();
        let mut visible: Vec<Sound> = sounds
            .iter()
            .filter(|s| {
                (q.is_empty()
                    || sound_label(s).to_lowercase().contains(&q)
                    || display_name(&s.name).to_lowercase().contains(&q))
                    && (!only_fav || !unfavorited.contains(&s.name))
            })
            .cloned()
            .collect();
        match sort.as_str() {
            "name-desc" => visible.sort_by(|a, b| b.name.cmp(&a.name)),
            "recent" => visible.sort_by(|a, b| {
                b.modified_secs
                    .cmp(&a.modified_secs)
                    .then_with(|| a.name.cmp(&b.name))
            }),
            _ => {}
        }

        let content: open_gpui::AnyElement = if sounds.is_empty() {
            empty_library(&lang, cx).into_any_element()
        } else if visible.is_empty() {
            no_results(&lang, &query, only_fav, cx).into_any_element()
        } else {
            self.sound_cards(
                &visible,
                &unfavorited,
                playing.as_ref(),
                compact,
                &lang,
                settings_open || about_open || browse_open || editor_open,
                cx,
            )
            .into_any_element()
        };

        // Rótulo da faixa em reprodução (display da metadata, se houver).
        let playing_label = playing
            .as_ref()
            .and_then(|name| sounds.iter().find(|s| &s.name == name))
            .map(sound_label);

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme::bg())
            .text_color(theme::text())
            .on_mouse_move(
                cx.listener(|this, event: &open_gpui::MouseMoveEvent, _window, cx| {
                    this.on_vol_move(f32::from(event.position.x), cx);
                    this.on_progress_move(f32::from(event.position.x), cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, window, cx| {
                    this.blur_search(window, cx);
                    this.blur_browse(window, cx);
                    this.blur_editor(window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event, _window, _cx| {
                    this.end_vol_drag();
                    this.end_progress_drag();
                }),
            )
            .on_drop(
                cx.listener(|this, paths: &open_gpui::ExternalPaths, _window, cx| {
                    this.import_dropped(paths.paths(), cx);
                }),
            )
            .child(titlebar(window, cx))
            .child(self.toolbar(
                sounds.len(),
                visible.len(),
                &query,
                search_focused,
                show_hint,
                only_fav,
                compact,
                settings_open,
                browse_open,
                &sort,
                sort_open,
                &lang,
                cx,
            ))
            .child(hint_banner(&lang, &shortcut, show_hint, cx))
            .child(status_banner(last_error))
            .child(
                div()
                    .id("library-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .px_4()
                    .pb_2()
                    .child(content)
                    // Respiro final: com o player flutuante, a última
                    // fileira precisa rolar para cima dele.
                    .child(
                        div().h(px(super::player::PLAYER_CLEARANCE)).flex_shrink_0(),
                    ),
            )
            .child(super::player::bottom_fade())
            .child(self.bottom_stack(
                playing.as_ref(),
                playing_label.as_deref(),
                volume,
                muted,
                playback,
                &play_peaks,
                play_paused,
                &lang,
                cx,
            ))
            .child(self.settings_overlay(cx))
            .child(self.browse_overlay(cx))
            .child(self.editor_dialog(cx))
            .child(drop_overlay(&lang, drop_active))
            .child(self.about_overlay(cx))
            .child(drop_overlay(&lang, drop_active))
            .children(resize_handles(cx))
    }
}

/// Alças de resize client-side.
///
/// Com `WindowDecorations::Client` (ver `main.rs`) o compositor não desenha
/// bordas de resize — sem isso a janela parecia "não redimensionável".
/// Cada borda/canto chama `start_window_resize`, igual ao exemplo
/// `window_shadow` do GPUI.
fn resize_handles(cx: &mut Context<MainWindow>) -> Vec<open_gpui::AnyElement> {
    const T: f32 = 8.0;
    const C: f32 = 16.0;
    let edge = |id: &'static str,
                edge: ResizeEdge,
                cursor: CursorStyle,
                pos: open_gpui::Div|
     -> open_gpui::AnyElement {
        pos.id(id)
            .cursor(cursor)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_this, _e, window, _cx| {
                    window.start_window_resize(edge);
                }),
            )
            .into_any_element()
    };
    vec![
        // Bordas recuadas nos cantos: as faixas iam de ponta a ponta e
        // roubavam o clique dos cantos (resize 1D em vez de 2D). Cada
        // canto de 16px pertence só à sua alça diagonal.
        edge(
            "resize-top",
            ResizeEdge::Top,
            CursorStyle::ResizeUpDown,
            div()
                .absolute()
                .top(px(0.0))
                .left(px(C))
                .right(px(C))
                .h(px(T)),
        ),
        edge(
            "resize-bottom",
            ResizeEdge::Bottom,
            CursorStyle::ResizeUpDown,
            div()
                .absolute()
                .bottom(px(0.0))
                .left(px(C))
                .right(px(C))
                .h(px(T)),
        ),
        edge(
            "resize-left",
            ResizeEdge::Left,
            CursorStyle::ResizeLeftRight,
            div()
                .absolute()
                .left(px(0.0))
                .top(px(C))
                .bottom(px(C))
                .w(px(T)),
        ),
        edge(
            "resize-right",
            ResizeEdge::Right,
            CursorStyle::ResizeLeftRight,
            div()
                .absolute()
                .right(px(0.0))
                .top(px(C))
                .bottom(px(C))
                .w(px(T)),
        ),
        // Cantos (área maior para pegar fácil)
        edge(
            "resize-tl",
            ResizeEdge::TopLeft,
            CursorStyle::ResizeUpLeftDownRight,
            div()
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .w(px(C))
                .h(px(C)),
        ),
        edge(
            "resize-tr",
            ResizeEdge::TopRight,
            CursorStyle::ResizeUpRightDownLeft,
            div()
                .absolute()
                .top(px(0.0))
                .right(px(0.0))
                .w(px(C))
                .h(px(C)),
        ),
        edge(
            "resize-bl",
            ResizeEdge::BottomLeft,
            CursorStyle::ResizeUpRightDownLeft,
            div()
                .absolute()
                .bottom(px(0.0))
                .left(px(0.0))
                .w(px(C))
                .h(px(C)),
        ),
        edge(
            "resize-br",
            ResizeEdge::BottomRight,
            CursorStyle::ResizeUpLeftDownRight,
            div()
                .absolute()
                .bottom(px(0.0))
                .right(px(0.0))
                .w(px(C))
                .h(px(C)),
        ),
    ]
}

/// Mapeia x em px de janela para fração 0–1 dentro de `bounds`.
/// Pura para ser testável (o `vol_track` é medido no layout via
/// `measured_element` e lido em `vol_fraction_at`).
fn vol_fraction_in(bounds: open_gpui::Bounds<open_gpui::Pixels>, x: f32) -> Option<f32> {
    let w = f32::from(bounds.size.width);
    if w <= 0.0 {
        return None;
    }
    Some(((x - f32::from(bounds.origin.x)) / w).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::vol_fraction_in;

    fn bounds_at(x: f32, w: f32) -> open_gpui::Bounds<open_gpui::Pixels> {
        open_gpui::Bounds {
            origin: open_gpui::point(open_gpui::px(x), open_gpui::px(0.0)),
            size: open_gpui::size(open_gpui::px(w), open_gpui::px(20.0)),
        }
    }

    #[test]
    fn click_maps_to_fraction() {
        let track = bounds_at(490.0, 212.0);
        // 100% -> 1.0 ; 0% -> 0.0 ; fora prende nas pontas.
        assert_eq!(vol_fraction_in(track, 490.0), Some(0.0));
        assert_eq!(vol_fraction_in(track, 702.0), Some(1.0));
        assert_eq!(vol_fraction_in(track, 200.0), Some(0.0));
        assert_eq!(vol_fraction_in(track, 900.0), Some(1.0));
        // Meio da trilha ~= 0.5 (tolerância de float).
        let mid = vol_fraction_in(track, 596.0).unwrap();
        assert!((mid - 0.5).abs() < 1e-6, "meio: {mid}");
        // Largura zerada não mapeia.
        assert_eq!(vol_fraction_in(bounds_at(0.0, 0.0), 10.0), None);
    }
}
