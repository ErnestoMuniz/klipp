import { app, type BrowserWindow } from "electron";
import fs from "node:fs/promises";
import path from "node:path";

const WINDOW_STATE_FILE = "window-state.json";

export interface WindowState {
  width: number;
  height: number;
  maximized: boolean;
}

export const DEFAULT_WINDOW_STATE: WindowState = {
  width: 900,
  height: 720,
  maximized: false,
};

function statePath(): string {
  return path.join(app.getPath("userData"), WINDOW_STATE_FILE);
}

function normalizeDimension(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? Math.round(value)
    : fallback;
}

export function normalizeWindowState(parsed: Partial<WindowState> | null): WindowState {
  return {
    width: normalizeDimension(parsed?.width, DEFAULT_WINDOW_STATE.width),
    height: normalizeDimension(parsed?.height, DEFAULT_WINDOW_STATE.height),
    maximized:
      typeof parsed?.maximized === "boolean" ? parsed.maximized : DEFAULT_WINDOW_STATE.maximized,
  };
}

export async function loadWindowState(): Promise<WindowState> {
  try {
    const raw = await fs.readFile(statePath(), "utf-8");
    return normalizeWindowState(JSON.parse(raw) as Partial<WindowState>);
  } catch {
    return DEFAULT_WINDOW_STATE;
  }
}

async function persistWindowState(state: WindowState): Promise<void> {
  await fs.mkdir(path.dirname(statePath()), { recursive: true });
  await fs.writeFile(statePath(), JSON.stringify(state, null, 2), "utf-8");
}

/** Keep the main window's normal size and maximized state in userData. */
export function trackWindowState(window: BrowserWindow, initialState: WindowState): void {
  let state = initialState;
  let saveTimer: NodeJS.Timeout | undefined;

  const saveSoon = (): void => {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = undefined;
      void persistWindowState(state).catch((error: unknown) => {
        console.warn("Unable to persist window state.", error);
      });
    }, 200);
  };

  window.on("resize", () => {
    if (!window.isMaximized() && !window.isMinimized()) {
      const [width, height] = window.getSize();
      state = { ...state, width, height };
      saveSoon();
    }
  });
  window.on("maximize", () => {
    state = { ...state, maximized: true };
    saveSoon();
  });
  window.on("unmaximize", () => {
    state = { ...state, maximized: false };
    saveSoon();
  });
  window.on("closed", () => {
    if (saveTimer) clearTimeout(saveTimer);
    void persistWindowState(state).catch((error: unknown) => {
      console.warn("Unable to persist window state.", error);
    });
  });
}
