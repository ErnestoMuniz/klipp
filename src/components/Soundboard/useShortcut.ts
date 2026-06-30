import { useCallback, useEffect, useState } from "react";
import { DEFAULT_SHORTCUT } from "./types";
import type { ShortcutApplyResult } from "../../audio-globals";

/**
 * Tracks the active global shortcut string. The source of truth lives in the
 * main process (persisted in userData); this hook fetches it on mount and
 * keeps in sync with `shortcut:changed` broadcasts so every renderer window —
 * including the overlay — shows the same binding.
 */
export function useShortcut(): {
  shortcut: string;
  set: (accelerator: string) => Promise<ShortcutApplyResult>;
} {
  const [shortcut, setShortcut] = useState<string>(DEFAULT_SHORTCUT);

  useEffect(() => {
    const api = window.shortcut;
    const ipc = window.ipcRenderer;
    if (!api) return;
    let active = true;
    void api
      .get()
      .then((value) => {
        if (active) setShortcut(value);
      })
      .catch(() => {
        /* leave default in place if the main process can't answer */
      });

    const handler = (_event: unknown, ...args: unknown[]) => {
      const value = args[0];
      if (typeof value === "string") setShortcut(value);
    };
    ipc?.on("shortcut:changed", handler);
    return () => {
      active = false;
      ipc?.off("shortcut:changed", handler);
    };
  }, []);

  const set = useCallback(
    async (accelerator: string): Promise<ShortcutApplyResult> => {
      const api = window.shortcut;
      if (!api) return { shortcut, registered: false, error: "invalid" };
      const result = await api.set(accelerator);
      if (result.registered) setShortcut(result.shortcut);
      return result;
    },
    [shortcut],
  );

  return { shortcut, set };
}
