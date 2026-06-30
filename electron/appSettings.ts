import { app, ipcMain, BrowserWindow } from "electron";
import fs from "node:fs/promises";
import path from "node:path";

// ---------------------------------------------------------------------------
// App-level settings storage
// ---------------------------------------------------------------------------
// App-wide preferences that the main process needs to act on (e.g. whether
// closing the main window hides it or quits the app) live in userData so they
// survive restarts. The renderer reads/writes them over IPC and stays in sync
// with `app-settings:changed` broadcasts.

const SETTINGS_FILE = "app-settings.json";

export interface AppSettings {
  /**
   * When true, closing the main window hides it to the tray and the app keeps
   * running in the background. When false, closing the main window quits the
   * app entirely.
   */
  runInBackground: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  runInBackground: true,
};

/** Merge a parsed-but-untrusted object onto the defaults. */
export function normalizeSettings(parsed: Partial<AppSettings> | null): AppSettings {
  return {
    runInBackground:
      typeof parsed?.runInBackground === "boolean"
        ? parsed.runInBackground
        : DEFAULT_SETTINGS.runInBackground,
  };
}

function settingsPath(): string {
  return path.join(app.getPath("userData"), SETTINGS_FILE);
}

/** Load the persisted settings, falling back to the defaults. */
export async function loadAppSettings(): Promise<AppSettings> {
  try {
    const raw = await fs.readFile(settingsPath(), "utf-8");
    return normalizeSettings(JSON.parse(raw) as Partial<AppSettings>);
  } catch {
    return DEFAULT_SETTINGS;
  }
}

async function persistSettings(settings: AppSettings): Promise<void> {
  await fs.mkdir(path.dirname(settingsPath()), { recursive: true });
  await fs.writeFile(settingsPath(), JSON.stringify(settings, null, 2), "utf-8");
}

function broadcast(settings: AppSettings): void {
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) window.webContents.send("app-settings:changed", settings);
  }
}

interface AppSettingsIpcHandlers {
  getCurrent: () => AppSettings;
  onApply: (next: AppSettings) => void;
}

/**
 * Register the `app-settings:get` / `app-settings:set` IPC handlers. `onApply`
 * is expected to update the main process's live state; on success the merged
 * settings are persisted and every renderer is notified via
 * `app-settings:changed`.
 */
export function registerSettingsIpc(handlers: AppSettingsIpcHandlers): void {
  ipcMain.handle("app-settings:get", () => handlers.getCurrent());
  ipcMain.handle("app-settings:set", async (_event, patch: unknown): Promise<AppSettings> => {
    const merged = normalizeSettings({
      ...handlers.getCurrent(),
      ...(patch as Partial<AppSettings>),
    });
    handlers.onApply(merged);
    await persistSettings(merged);
    broadcast(merged);
    return merged;
  });
}
