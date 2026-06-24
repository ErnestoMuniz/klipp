import {
  app,
  BrowserWindow,
  Menu,
  Tray,
  globalShortcut,
  ipcMain,
  nativeImage,
  screen,
  type Display,
} from "electron";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { uIOhook, UiohookKey, type UiohookKeyboardEvent } from "uiohook-napi";
import { AudioManager, registerAudioIpc } from "./audio";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

app.commandLine.appendSwitch("enable-features", "GlobalShortcutsPortal");

process.env.APP_ROOT = path.join(__dirname, "..");

const VITE_DEV_SERVER_URL = process.env["VITE_DEV_SERVER_URL"];
const RENDERER_DIST = path.join(process.env.APP_ROOT, "dist");

process.env.VITE_PUBLIC = VITE_DEV_SERVER_URL
  ? path.join(process.env.APP_ROOT, "public")
  : RENDERER_DIST;

process.env.PULSE_SINK = "soundboard-clips";

const GLOBAL_SHORTCUT = "Alt+Shift+S";

// ============================================================================
// ESTADO GLOBAL
// ============================================================================

const audio = new AudioManager();

let win: BrowserWindow | null = null;
let tray: Tray | null = null;
let isQuitting = false;
let shortcutHeld = false;
let overlayActive = false;
let activeOverlayDisplayId: number | null = null;

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
  win = new BrowserWindow({
    icon: path.join(process.env.VITE_PUBLIC!, "favicon.svg"),
    width: 900,
    height: 720,
    webPreferences: {
      preload: path.join(__dirname, "preload.mjs"),
      sandbox: false,
      backgroundThrottling: false,
    },
  });

  win.on("close", (event) => {
    if (!isQuitting) {
      event.preventDefault();
      win?.hide();
    }
  });

  win.webContents.on("did-finish-load", () => {
    win?.webContents.send("main-process-message", new Date().toLocaleString());
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
  const iconPath = path.join(process.env.VITE_PUBLIC!, "tray-icon-32.png");
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
      overlayWindow.webContents.send("overlay:show", audio.getState().sounds);
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
    if (view.rendererReady) {
      view.window.webContents.send("overlay:show", audio.getState().sounds);
    }
    view.window.setSkipTaskbar(true);
    view.window.setFocusable(true);
    view.window.setIgnoreMouseEvents(false);
  }

  overlays.get(activeDisplay.id)?.window.focus();
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
    win?.webContents.send("main:sound-playing", sound.name);
    hideOverlays();
  });
  ipcMain.on("overlay:playback-ended", (event) => {
    if (!overlayForWebContents(event.sender.id)) return;
    win?.webContents.send("main:sound-playing", null);
  });
}

// ============================================================================
// ATALHO GLOBAL
// ============================================================================

function isShortcutKey(event: UiohookKeyboardEvent): boolean {
  return event.keycode === UiohookKey.S;
}

function isShortcutModifier(event: UiohookKeyboardEvent): boolean {
  return (
    event.keycode === UiohookKey.Alt ||
    event.keycode === UiohookKey.AltRight ||
    event.keycode === UiohookKey.Shift ||
    event.keycode === UiohookKey.ShiftRight
  );
}

function registerShortcut(): void {
  const registered = globalShortcut.register(GLOBAL_SHORTCUT, () => {
    shortcutHeld = true;
    showOverlay();
  });

  if (!registered) {
    console.error(`Não foi possível registrar o atalho global ${GLOBAL_SHORTCUT}.`);
  }

  uIOhook.on("keydown", (event) => {
    if (shortcutHeld || !isShortcutKey(event) || !event.altKey || !event.shiftKey) return;
    shortcutHeld = true;
    showOverlay();
  });

  uIOhook.on("keyup", (event) => {
    if (
      !shortcutHeld ||
      (!isShortcutKey(event) && !isShortcutModifier(event) && event.altKey && event.shiftKey)
    ) {
      return;
    }

    shortcutHeld = false;
    confirmOverlaySelection();
  });

  uIOhook.start();
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
  uIOhook.stop();
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

void app.whenReady().then(async () => {
  await audio.init();
  registerAudioIpc(audio);
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
});
