import { app, ipcMain, BrowserWindow } from "electron";
import fs from "node:fs/promises";
import path from "node:path";

// ---------------------------------------------------------------------------
// Global shortcut storage + parsing
// ---------------------------------------------------------------------------
// The quick-picker overlay is triggered by a global keyboard shortcut. The
// accelerator string (Electron format, e.g. "Alt+Shift+S") is persisted in
// userData so the user can rebind it from Settings and keep the change across
// restarts. The main process parses it into the modifier flags + uiohook
// keycode used by the low-level key listener that detects the shortcut
// release (to confirm a picker selection).

export const DEFAULT_SHORTCUT = "Alt+Shift+S";
const SHORTCUT_FILE = "shortcut.json";

/** Parsed accelerator: which modifiers must be held and which key codes fire. */
export interface ParsedAccelerator {
  alt: boolean;
  shift: boolean;
  ctrl: boolean;
  meta: boolean;
  /** uiohook keycode of the terminal key (e.g. UiohookKey.S). */
  key: number;
}

const LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");
const DIGITS = "0123456789".split("");
const FUNCTION_KEYS = Array.from({ length: 24 }, (_, index) => `F${index + 1}`);

// Keep these key codes local instead of importing `uiohook-napi` here. That
// package loads its native addon as soon as it is imported, which would make
// an unavailable X11 input backend abort the main process before Electron can
// create a window. The actual hook is loaded lazily (and guarded) in main.ts.
const codes: Record<string, number> = {
  0: 11,
  1: 2,
  2: 3,
  3: 4,
  4: 5,
  5: 6,
  6: 7,
  7: 8,
  8: 9,
  9: 10,
  A: 30,
  B: 48,
  C: 46,
  D: 32,
  E: 18,
  F: 33,
  G: 34,
  H: 35,
  I: 23,
  J: 36,
  K: 37,
  L: 38,
  M: 50,
  N: 49,
  O: 24,
  P: 25,
  Q: 16,
  R: 19,
  S: 31,
  T: 20,
  U: 22,
  V: 47,
  W: 17,
  X: 45,
  Y: 21,
  Z: 44,
  F1: 59,
  F2: 60,
  F3: 61,
  F4: 62,
  F5: 63,
  F6: 64,
  F7: 65,
  F8: 66,
  F9: 67,
  F10: 68,
  F11: 87,
  F12: 88,
  F13: 91,
  F14: 92,
  F15: 93,
  F16: 99,
  F17: 100,
  F18: 101,
  F19: 102,
  F20: 103,
  F21: 104,
  F22: 105,
  F23: 106,
  F24: 107,
};

const KEY_MAP: Record<string, number> = {};
for (const letter of LETTERS) KEY_MAP[letter] = codes[letter];
for (const digit of DIGITS) KEY_MAP[digit] = codes[digit];
for (const fn of FUNCTION_KEYS) KEY_MAP[fn] = codes[fn];

/**
 * Parse an Electron accelerator string into the parts the uiohook listener
 * needs. Returns `null` for anything we can't match (unsupported key, empty,
 * or unrecognised modifier tokens).
 */
export function parseAccelerator(accelerator: string): ParsedAccelerator | null {
  const tokens = accelerator
    .split("+")
    .map((token) => token.trim())
    .filter(Boolean);
  if (tokens.length === 0) return null;

  const keyToken = tokens[tokens.length - 1]!.toUpperCase();
  const key = KEY_MAP[keyToken];
  if (key === undefined) return null;

  let alt = false;
  let shift = false;
  let ctrl = false;
  let meta = false;

  for (let index = 0; index < tokens.length - 1; index++) {
    const mod = tokens[index]!.toLowerCase();
    switch (mod) {
      case "alt":
        alt = true;
        break;
      case "shift":
        shift = true;
        break;
      case "ctrl":
      case "control":
        ctrl = true;
        break;
      case "cmd":
      case "command":
      case "meta":
      case "super":
        meta = true;
        break;
      case "commandorctrl":
        // Electron resolves this per-platform; we approximate by requiring
        // both, which matches neither platform exactly but is only relevant
        // for the release-detection heuristic.
        ctrl = true;
        meta = true;
        break;
      default:
        return null;
    }
  }

  return { alt, shift, ctrl, meta, key };
}

function shortcutPath(): string {
  return path.join(app.getPath("userData"), SHORTCUT_FILE);
}

/** Load the persisted accelerator, falling back to the default. */
export async function loadStoredShortcut(): Promise<string> {
  try {
    const raw = await fs.readFile(shortcutPath(), "utf-8");
    const parsed = JSON.parse(raw) as { shortcut?: unknown };
    if (typeof parsed.shortcut === "string" && parseAccelerator(parsed.shortcut)) {
      return parsed.shortcut;
    }
  } catch {
    /* missing or corrupt file — fall through to default */
  }
  return DEFAULT_SHORTCUT;
}

async function persistShortcut(shortcut: string): Promise<void> {
  await fs.mkdir(path.dirname(shortcutPath()), { recursive: true });
  await fs.writeFile(shortcutPath(), JSON.stringify({ shortcut }, null, 2), "utf-8");
}

/** Result of applying a new shortcut. `error` is a stable machine code. */
export interface ShortcutApplyResult {
  shortcut: string;
  registered: boolean;
  error?: "invalid" | "taken";
}

interface ShortcutIpcHandlers {
  getCurrent: () => string;
  onApply: (accelerator: string) => ShortcutApplyResult;
}

function broadcast(shortcut: string): void {
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) window.webContents.send("shortcut:changed", shortcut);
  }
}

/**
 * Register the `shortcut:get` / `shortcut:set` IPC handlers. `onApply` is
 * expected to (re)register the Electron globalShortcut and update the main
 * process's parsed state; on success the shortcut is persisted and every
 * renderer is notified via `shortcut:changed`.
 */
export function registerShortcutIpc(handlers: ShortcutIpcHandlers): void {
  ipcMain.handle("shortcut:get", () => handlers.getCurrent());
  ipcMain.handle(
    "shortcut:set",
    async (_event, shortcut: unknown): Promise<ShortcutApplyResult> => {
      if (typeof shortcut !== "string" || !parseAccelerator(shortcut)) {
        return { shortcut: handlers.getCurrent(), registered: false, error: "invalid" };
      }
      const result = handlers.onApply(shortcut);
      if (result.registered) {
        await persistShortcut(shortcut);
        broadcast(shortcut);
      }
      return result;
    },
  );
}
