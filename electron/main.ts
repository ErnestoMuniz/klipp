import {
  app,
  BrowserWindow,
  Menu,
  Tray,
  globalShortcut,
  ipcMain,
  nativeImage,
  protocol,
  screen,
  shell,
  type Display,
} from "electron";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { UiohookKeyboardEvent } from "uiohook-napi";
import { AudioManager, registerAudioIpc, registerSoundProtocol, SOUND_SCHEME } from "./audio";
import { searchMyinstants } from "./myinstants";
import {
  DEFAULT_SHORTCUT,
  loadStoredShortcut,
  parseAccelerator,
  registerShortcutIpc,
  type ParsedAccelerator,
} from "./shortcut";
import {
  DEFAULT_SETTINGS,
  loadAppSettings,
  registerSettingsIpc,
  type AppSettings,
} from "./appSettings";
import {
  DEFAULT_WINDOW_STATE,
  loadWindowState,
  trackWindowState,
  type WindowState,
} from "./windowState";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

app.commandLine.appendSwitch("enable-features", "GlobalShortcutsPortal");

// Register the sound scheme as privileged before app ready so the renderer can
// use it as a secure, fetch/stream-capable protocol for playing sound files
// that live in userData (outside the read-only app bundle).
protocol.registerSchemesAsPrivileged([
  {
    scheme: SOUND_SCHEME,
    privileges: {
      standard: true,
      secure: true,
      supportFetchAPI: true,
      stream: true,
      codeCache: true,
    },
  },
]);

process.env.APP_ROOT = path.join(__dirname, "..");

const VITE_DEV_SERVER_URL = process.env["VITE_DEV_SERVER_URL"];
const RENDERER_DIST = path.join(process.env.APP_ROOT, "dist");

process.env.VITE_PUBLIC = VITE_DEV_SERVER_URL
  ? path.join(process.env.APP_ROOT, "public")
  : RENDERER_DIST;

process.env.PULSE_SINK = "klipp-clips";

// Currently bound global shortcut. Loaded from disk on startup and updated
// whenever a renderer rebind succeeds. The uIOhook listeners read
// `currentParsed` on every event so they always reflect the live binding.
let currentShortcut = DEFAULT_SHORTCUT;
let currentParsed: ParsedAccelerator | null = parseAccelerator(DEFAULT_SHORTCUT);
let currentPlayingUrl: string | null = null;

// ============================================================================
// ESTADO GLOBAL
// ============================================================================

const audio = new AudioManager();

let win: BrowserWindow | null = null;
let tray: Tray | null = null;
let isQuitting = false;
let shortcutHeld = false;
let nativeShortcutHookStarted = false;
let nativeShortcutHook: typeof import("uiohook-napi").uIOhook | null = null;
let nativeShortcutKeys: typeof import("uiohook-napi").UiohookKey | null = null;
let overlayActive = false;
let activeOverlayDisplayId: number | null = null;

// App-level preferences that the main process acts on. Loaded from disk on
// startup and updated live whenever the renderer changes them; the close
// handler reads `runInBackground` to decide whether to hide or quit.
let currentSettings: AppSettings = DEFAULT_SETTINGS;
let currentWindowState: WindowState = DEFAULT_WINDOW_STATE;

interface OverlayView {
  displayId: number;
  window: BrowserWindow;
  rendererReady: boolean;
}

const overlays = new Map<number, OverlayView>();

// ============================================================================
// JANELA PRINCIPAL + TRAY
// ============================================================================

function loadRenderer(window: BrowserWindow, query?: Record<string, string>): void {
  if (VITE_DEV_SERVER_URL) {
    const url = new URL(VITE_DEV_SERVER_URL);
    for (const [key, value] of Object.entries(query ?? {})) {
      url.searchParams.set(key, value);
    }
    void window.loadURL(url.toString());
  } else {
    void window.loadFile(path.join(RENDERER_DIST, "index.html"), { query });
  }
}

function createWindow(): void {
  const iconPath = path.join(process.env.VITE_PUBLIC!, "logo.png");
  const icon = nativeImage.createFromPath(iconPath);

  win = new BrowserWindow({
    icon,
    width: currentWindowState.width,
    height: currentWindowState.height,
    // Frameless window so the renderer owns the whole chrome (custom
    // macOS-style traffic-light buttons live in the Topbar). On macOS
    // `titleBarStyle: "hidden"" keeps a draggable title bar region but
    // removes the native title text + buttons; everywhere else `frame: false`
    // removes the system title bar entirely.
    frame: process.platform === "darwin" ? undefined : false,
    titleBarStyle: "hidden",
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(__dirname, "preload.mjs"),
      sandbox: false,
      backgroundThrottling: false,
    },
  });

  // No application menu, no visible menu bar — the app drives everything via
  // its own UI.
  win.setMenuBarVisibility(false);

  win.on("close", (event) => {
    if (isQuitting) return;
    if (currentSettings.runInBackground) {
      event.preventDefault();
      win?.hide();
      return;
    }
    // Background execution disabled: quit the whole app. Mark quitting so
    // the tray/overlay teardown doesn't fight the shutdown and `before-quit`
    // runs the cleanup pipeline (audio, shortcuts, hooks, overlays).
    isQuitting = true;
    app.quit();
  });

  trackWindowState(win, currentWindowState);
  win.on("maximize", () => win?.webContents.send("window:maximized", true));
  win.on("unmaximize", () => win?.webContents.send("window:maximized", false));

  if (currentWindowState.maximized) win.maximize();

  win.webContents.on("did-finish-load", () => {
    win?.webContents.send("main-process-message", new Date().toLocaleString());
    win?.webContents.send("window:maximized", win?.isMaximized() ?? false);
  });

  loadRenderer(win);
}

function showWindow() {
  if (win) {
    if (win.isMinimized()) win.restore();
    win.show();
    win.focus();
  } else {
    createWindow();
  }
}

function createTray() {
  const iconPath = path.join(process.env.VITE_PUBLIC!, "tray-32.png");
  const icon = nativeImage.createFromPath(iconPath);
  tray = new Tray(icon.isEmpty() ? nativeImage.createEmpty() : icon);

  const menu = Menu.buildFromTemplate([
    { label: "Klipp", enabled: false },
    { type: "separator" },
    { label: "Mostrar", click: () => showWindow() },
    { label: "Sair", click: () => quitApp() },
  ]);

  tray.setContextMenu(menu);
  tray.setToolTip("Klipp — clique para abrir");
  tray.on("click", () => showWindow());
}

function quitApp() {
  isQuitting = true;
  app.quit();
}

// ============================================================================
// OVERLAY ELECTRON
// ============================================================================

function hideOverlays(): void {
  overlayActive = false;
  activeOverlayDisplayId = null;
  for (const { window } of overlays.values()) {
    if (window.isDestroyed()) continue;

    window.webContents.send("overlay:hide");
    window.setIgnoreMouseEvents(true);
    window.setFocusable(false);
    window.setSkipTaskbar(true);
  }
}

function createOverlay(display: Display): OverlayView {
  const existing = overlays.get(display.id);
  if (existing && !existing.window.isDestroyed()) return existing;

  const overlayWindow = new BrowserWindow({
    ...display.bounds,
    show: false,
    frame: false,
    transparent: true,
    resizable: false,
    movable: false,
    minimizable: false,
    maximizable: false,
    fullscreenable: false,
    skipTaskbar: true,
    focusable: false,
    type: process.platform === "linux" ? "toolbar" : undefined,
    title: "",
    alwaysOnTop: true,
    hasShadow: false,
    backgroundColor: "#00000000",
    webPreferences: {
      preload: path.join(__dirname, "preload.mjs"),
      sandbox: false,
      // O áudio deve continuar mesmo depois que o seletor é escondido.
      backgroundThrottling: false,
    },
  });

  const view: OverlayView = {
    displayId: display.id,
    window: overlayWindow,
    rendererReady: false,
  };
  overlays.set(display.id, view);

  overlayWindow.setMenuBarVisibility(false);
  overlayWindow.setIgnoreMouseEvents(true);
  overlayWindow.setSkipTaskbar(true);
  overlayWindow.setFocusable(false);
  overlayWindow.on("blur", hideOverlays);
  overlayWindow.on("closed", () => {
    const current = overlays.get(display.id);
    if (current?.window === overlayWindow) overlays.delete(display.id);
  });
  overlayWindow.webContents.on("render-process-gone", hideOverlays);
  overlayWindow.webContents.on("did-finish-load", () => {
    view.rendererReady = true;
    overlayWindow.setIgnoreMouseEvents(true);
    overlayWindow.setFocusable(false);
    overlayWindow.setSkipTaskbar(true);
    overlayWindow.showInactive();
    overlayWindow.setFocusable(false);
    overlayWindow.setSkipTaskbar(true);

    if (overlayActive) {
      overlayWindow.webContents.send("overlay:show", audio.overlaySounds(), currentPlayingUrl);
      overlayWindow.setFocusable(true);
      overlayWindow.setIgnoreMouseEvents(false);
    } else {
      overlayWindow.webContents.send("overlay:hide");
    }
  });

  loadRenderer(overlayWindow, { overlay: "1", display: String(display.id) });
  return view;
}

function syncOverlays(): void {
  const displays = screen.getAllDisplays();
  const activeIds = new Set(displays.map((display) => display.id));

  for (const [displayId, view] of overlays) {
    if (activeIds.has(displayId)) continue;
    view.window.destroy();
    overlays.delete(displayId);
  }

  for (const display of displays) {
    if (!overlays.has(display.id)) createOverlay(display);
  }
}

function recreateOverlay(display: Display): void {
  const current = overlays.get(display.id);
  current?.window.destroy();
  overlays.delete(display.id);
  createOverlay(display);
}

function showOverlay(): void {
  syncOverlays();

  if (overlayActive) return;

  overlayActive = true;
  const activeDisplay = screen.getDisplayNearestPoint(screen.getCursorScreenPoint());
  activeOverlayDisplayId = activeDisplay.id;
  for (const view of overlays.values()) {
    if (view.window.isDestroyed()) continue;
    if (!view.rendererReady) {
      // A transparent window without a renderer must remain click-through.
      // Otherwise a dev-server failure leaves an invisible fullscreen window
      // intercepting the desktop until the application is terminated.
      view.window.setIgnoreMouseEvents(true);
      view.window.setFocusable(false);
      continue;
    }
    view.window.webContents.send("overlay:show", audio.overlaySounds(), currentPlayingUrl);
    view.window.setSkipTaskbar(true);
    view.window.setFocusable(true);
    view.window.setIgnoreMouseEvents(false);
  }

  const activeOverlay = overlays.get(activeDisplay.id);
  if (!activeOverlay?.rendererReady || activeOverlay.window.isDestroyed()) {
    hideOverlays();
    return;
  }
  activeOverlay.window.focus();
}

function confirmOverlaySelection(): void {
  if (!overlayActive || activeOverlayDisplayId === null) {
    hideOverlays();
    return;
  }

  const activeOverlay = overlays.get(activeOverlayDisplayId);
  if (!activeOverlay?.rendererReady || activeOverlay.window.isDestroyed()) {
    hideOverlays();
    return;
  }

  activeOverlay.window.webContents.send("overlay:confirm-selection");
}

function overlayForWebContents(webContentsId: number): OverlayView | undefined {
  return [...overlays.values()].find(({ window }) => window.webContents.id === webContentsId);
}

function registerOverlayIpc(): void {
  ipcMain.on("overlay:pointer-ready", (event) => {
    const selectedOverlay = overlayForWebContents(event.sender.id);
    if (!selectedOverlay || !overlayActive) return;

    activeOverlayDisplayId = selectedOverlay.displayId;
    for (const view of overlays.values()) {
      if (view === selectedOverlay || view.window.isDestroyed()) continue;
      view.window.webContents.send("overlay:hide");
      view.window.setIgnoreMouseEvents(true);
      view.window.setFocusable(false);
      view.window.setSkipTaskbar(true);
    }
    selectedOverlay.window.setSkipTaskbar(true);
    selectedOverlay.window.focus();
  });
  ipcMain.on("overlay:hide", (event) => {
    if (overlayForWebContents(event.sender.id)) hideOverlays();
  });
  ipcMain.on("overlay:selected", (event, url: string) => {
    const selectedOverlay = overlayForWebContents(event.sender.id);
    if (!selectedOverlay) return;
    const sound = audio.getState().sounds.find((candidate) => candidate.url === url);
    if (!sound) return;

    for (const view of overlays.values()) {
      if (view === selectedOverlay || view.window.isDestroyed()) continue;
      view.window.webContents.send("overlay:stop-playback");
    }
    win?.webContents.send("main:stop-playback");
    currentPlayingUrl = sound.url;
    win?.webContents.send("main:sound-playing", sound.url);
    hideOverlays();
  });
  ipcMain.on("overlay:playback-ended", (event) => {
    if (!overlayForWebContents(event.sender.id)) return;
    currentPlayingUrl = null;
    win?.webContents.send("main:stop-playback");
    win?.webContents.send("main:sound-playing", null);
  });
  ipcMain.on("main:playback-started", (event, url: string) => {
    if (event.sender !== win?.webContents) return;
    for (const view of overlays.values()) {
      if (!view.window.isDestroyed()) view.window.webContents.send("overlay:stop-playback");
    }
    currentPlayingUrl = url;
    win?.webContents.send("main:sound-playing", url);
  });
  ipcMain.on("main:playback-ended", (event) => {
    if (event.sender !== win?.webContents) return;
    for (const view of overlays.values()) {
      if (!view.window.isDestroyed()) view.window.webContents.send("overlay:stop-playback");
    }
    currentPlayingUrl = null;
    win?.webContents.send("main:sound-playing", null);
  });
}

// ============================================================================
// ATALHO GLOBAL
// ============================================================================

function matchesModifiers(event: UiohookKeyboardEvent, parsed: ParsedAccelerator): boolean {
  return (
    event.altKey === parsed.alt &&
    event.shiftKey === parsed.shift &&
    event.ctrlKey === parsed.ctrl &&
    event.metaKey === parsed.meta
  );
}

function isShortcutModifierKey(event: UiohookKeyboardEvent, parsed: ParsedAccelerator): boolean {
  const keys = nativeShortcutKeys;
  if (!keys) return false;

  if (parsed.alt && (event.keycode === keys.Alt || event.keycode === keys.AltRight)) {
    return true;
  }
  if (parsed.shift && (event.keycode === keys.Shift || event.keycode === keys.ShiftRight)) {
    return true;
  }
  if (parsed.ctrl && (event.keycode === keys.Ctrl || event.keycode === keys.CtrlRight)) {
    return true;
  }
  if (parsed.meta && (event.keycode === keys.Meta || event.keycode === keys.MetaRight)) {
    return true;
  }
  return false;
}

/**
 * Apply a new accelerator as the active global shortcut. Registers the
 * Electron global accelerator and updates the parsed state the uIOhook
 * listeners read. On failure the previous binding is restored so the overlay
 * keeps working.
 */
function applyShortcut(accelerator: string): {
  shortcut: string;
  registered: boolean;
  error?: "invalid" | "taken";
} {
  const parsed = parseAccelerator(accelerator);
  if (!parsed) return { shortcut: currentShortcut, registered: false, error: "invalid" };

  globalShortcut.unregisterAll();
  const registered = globalShortcut.register(accelerator, () => {
    shortcutHeld = true;
    showOverlay();
  });

  if (!registered) {
    // Restore the previous binding so the overlay stays usable.
    globalShortcut.register(currentShortcut, () => {
      shortcutHeld = true;
      showOverlay();
    });
    return { shortcut: currentShortcut, registered: false, error: "taken" };
  }

  currentShortcut = accelerator;
  currentParsed = parsed;
  return { shortcut: accelerator, registered: true };
}

async function startNativeShortcutHook(): Promise<void> {
  // Flatpak deliberately isolates applications from the host's raw input
  // devices/XRecord backend. uIOhook can block synchronously while trying to
  // initialise that backend, freezing the Electron main loop after the native
  // window has been created but before its renderer paints. Inside Flatpak the
  // Electron globalShortcut implementation uses the GlobalShortcuts portal.
  if (process.env.FLATPAK_ID) {
    console.info("Flatpak detected; using the GlobalShortcuts portal.");
    return;
  }

  try {
    // An unavailable native input backend must never prevent Electron from
    // showing the main window in a sandboxed session.
    const { uIOhook, UiohookKey } = await import("uiohook-napi");
    nativeShortcutHook = uIOhook;
    nativeShortcutKeys = UiohookKey;

    uIOhook.on("keydown", (event) => {
      if (shortcutHeld || !currentParsed) return;
      if (!matchesModifiers(event, currentParsed) || event.keycode !== currentParsed.key) return;
      shortcutHeld = true;
      showOverlay();
    });

    uIOhook.on("keyup", (event) => {
      if (!shortcutHeld || !currentParsed) return;
      if (event.keycode !== currentParsed.key && !isShortcutModifierKey(event, currentParsed)) {
        return;
      }
      shortcutHeld = false;
      confirmOverlaySelection();
    });

    uIOhook.start();
    nativeShortcutHookStarted = true;
  } catch (error) {
    // This can happen in a sandboxed Wayland session when direct input access
    // is unavailable. Electron's globalShortcut registration above remains a
    // usable fallback, so never make the whole application fail to start.
    console.warn(
      "Native global shortcut hook unavailable; using Electron shortcut fallback.",
      error,
    );
  }
}

function registerShortcut(): void {
  const registered = globalShortcut.register(currentShortcut, () => {
    shortcutHeld = true;
    showOverlay();
  });

  if (!registered) {
    console.error(`Não foi possível registrar o atalho global ${currentShortcut}.`);
  }

  void startNativeShortcutHook();
  registerShortcutIpc({ getCurrent: () => currentShortcut, onApply: applyShortcut });
}

// ============================================================================
// CICLO DE VIDA DO APP
// ============================================================================

app.on("window-all-closed", () => {});

app.on("activate", () => showWindow());

let cleaningUp = false;
function cleanupAndExit(exitCode = 0): void {
  if (cleaningUp) return;
  cleaningUp = true;
  globalShortcut.unregisterAll();
  if (nativeShortcutHookStarted && nativeShortcutHook) {
    try {
      nativeShortcutHook.stop();
    } catch (error) {
      console.warn("Unable to stop native global shortcut hook.", error);
    }
  }
  hideOverlays();
  void audio.cleanup().finally(() => process.exit(exitCode));
}

app.on("before-quit", (event) => {
  event.preventDefault();
  cleanupAndExit(0);
});

process.on("SIGINT", () => {
  isQuitting = true;
  cleanupAndExit(0);
});
process.on("SIGTERM", () => {
  isQuitting = true;
  cleanupAndExit(0);
});

// Frameless main window: custom macOS-style traffic-light controls live in
// the renderer Topbar, so drive minimize / maximize / close from there.
ipcMain.on("window:minimize", () => win?.minimize());
ipcMain.on("window:toggle-maximize", () => {
  if (!win) return;
  if (win.isMaximized()) win.unmaximize();
  else win.maximize();
});
ipcMain.on("window:close", () => {
  if (!win) return;
  isQuitting = false;
  win.close();
});
ipcMain.handle("app-info:get-version", () => app.getVersion());
ipcMain.handle("app-info:open-external", (_event, url: unknown) => {
  if (url === "https://github.com/ErnestoMuniz/klipp") return shell.openExternal(url);
});

// Set app name early so Linux window managers can match the .desktop file icon
app.setName("klipp");

const hasSingleInstanceLock = app.requestSingleInstanceLock();
if (!hasSingleInstanceLock) {
  app.exit(0);
}

app.on("second-instance", () => {
  if (app.isReady()) showWindow();
});

void app.whenReady().then(async () => {
  // No application menu, no visible menu bar — the UI owns all chrome.
  Menu.setApplicationMenu(null);

  // Load the persisted global shortcut before wiring up its listeners so the
  // first registration uses the user's chosen binding.
  currentShortcut = await loadStoredShortcut();
  currentParsed = parseAccelerator(currentShortcut);

  // Load app-level preferences before creating windows so the close handler
  // already reflects the user's "run in background" choice on first close.
  currentSettings = await loadAppSettings();
  currentWindowState = await loadWindowState();

  registerSoundProtocol();
  registerAudioIpc(audio);

  // Renderer side of myinstants browse: forward searches to the scraper. Done
  // in the main process so the renderer avoids CORS and HTML parsing noise.
  ipcMain.handle("myinstants:search", async (_event, query: string, page?: number) =>
    searchMyinstants(query, page ?? 1),
  );
  registerSettingsIpc({
    getCurrent: () => currentSettings,
    onApply: (next) => {
      currentSettings = next;
    },
  });
  createWindow();
  syncOverlays();
  screen.on("display-added", (_event, display) => createOverlay(display));
  screen.on("display-removed", (_event, display) => {
    overlays.get(display.id)?.window.destroy();
    overlays.delete(display.id);
  });
  screen.on("display-metrics-changed", (_event, display) => recreateOverlay(display));
  createTray();
  registerOverlayIpc();
  registerShortcut();

  // Audio routing invokes host PulseAudio/PipeWire commands in the Flatpak
  // build. Do not make that work a prerequisite for showing the app window.
  void audio.init().finally(() => {
    win?.webContents.send("audio:state-changed", audio.getState());
    void audio
      .watchSounds((state) => {
        const window = win;
        if (window && !window.isDestroyed()) {
          window.webContents.send("audio:state-changed", state);
        }
      })
      .catch(() => {});
  });
});
