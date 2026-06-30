import type { AudioState } from "../../audio-globals";
import type { SoundFile } from "../../audio-globals";

export type Status = "idle" | "busy";
export type Density = "comfort" | "compact";
export type SortMode = "name-asc" | "name-desc" | "recent";
export type Theme = "light" | "dark" | "system";
/** Library-level view filter. "overlay" filters to sounds in the quick picker. */
export type ViewFilter = "all" | "overlay";

export const DEFAULT_SHORTCUT = "Alt+Shift+S";
export const PREFS_KEY = "klipp.prefs.v1";

export interface Prefs {
  density: Density;
  sort: SortMode;
  showHints: boolean;
  volume: number;
  muted: boolean;
  theme: Theme;
  /** Library-only filter that hides sounds not added to the overlay picker. */
  onlyOverlay: boolean;
}

export const defaultPrefs: Prefs = {
  density: "comfort",
  sort: "name-asc",
  showHints: true,
  volume: 1,
  muted: false,
  theme: "system",
  onlyOverlay: false,
};

export const emptyState: AudioState = {
  ready: false,
  error: null,
  virtualMic: "soundboard-mic",
  discordDeviceName: "Soundboard-Mic",
  clipsSink: "soundboard-clips",
  mixSink: "soundboard-mix",
  micSources: [],
  currentMicSource: null,
  micPassthrough: false,
  hearClips: true,
  defaultSink: "",
  sounds: [],
};

export function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (!raw) return defaultPrefs;
    const parsed = JSON.parse(raw) as Partial<Prefs>;
    return { ...defaultPrefs, ...parsed };
  } catch {
    return defaultPrefs;
  }
}

export function labelFor(name: string): string {
  return name.replace(/\.[^.]+$/, "");
}

export function soundLabel(sound: SoundFile): string {
  return sound.displayName || labelFor(sound.name);
}

export function initials(name: string): string {
  const label = labelFor(name);
  const parts = label.split(/[\s_-]+/).filter(Boolean);
  if (parts.length === 0) return "♪";
  if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase();
  return (parts[0]![0]! + parts[1]![0]!).toUpperCase();
}
