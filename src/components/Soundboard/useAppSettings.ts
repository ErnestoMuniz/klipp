import { useCallback, useEffect, useState } from "react";
import type { AppSettings } from "../../audio-globals";

/**
 * Tracks the app-wide settings (e.g. whether the app keeps running in the
 * background after the main window is closed). The source of truth lives in
 * the main process, persisted in userData; this hook fetches it on mount and
 * stays in sync with `app-settings:changed` broadcasts so every renderer
 * window reflects the same state.
 */
export function useAppSettings(): {
  settings: AppSettings;
  set: (patch: Partial<AppSettings>) => Promise<void>;
  ready: boolean;
} {
  const [settings, setSettings] = useState<AppSettings>({ runInBackground: true });
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const api = window.appSettings;
    const ipc = window.ipcRenderer;
    if (!api) return;
    let active = true;
    void api
      .get()
      .then((value) => {
        if (active) {
          setSettings(value);
          setReady(true);
        }
      })
      .catch(() => {
        /* leave the default in place if the main process can't answer */
      });

    const handler = (_event: unknown, ...args: unknown[]) => {
      const value = args[0] as AppSettings | undefined;
      if (value) setSettings(value);
    };
    ipc?.on("app-settings:changed", handler);
    return () => {
      active = false;
      ipc?.off("app-settings:changed", handler);
    };
  }, []);

  const set = useCallback(async (patch: Partial<AppSettings>) => {
    const api = window.appSettings;
    if (!api) return;
    const next = await api.set(patch);
    setSettings(next);
  }, []);

  return { settings, set, ready };
}
