export interface MicSource {
  name: string;
  description: string;
}

export interface SoundFile {
  emoji: string;
  displayName: string;
  name: string;
  url: string;
  /** Whether this sound is selectable in the quick overlay picker. */
  inOverlay: boolean;
}

export interface SoundMetadata {
  emoji?: string;
  displayName?: string;
  /** New value for the overlay-availability flag (preserved when omitted). */
  inOverlay?: boolean;
}

export interface AudioState {
  ready: boolean;
  error: string | null;
  virtualMic: string;
  discordDeviceName: string;
  clipsSink: string;
  mixSink: string;
  micSources: MicSource[];
  currentMicSource: string | null;
  micPassthrough: boolean;
  hearClips: boolean;
  defaultSink: string;
  defaultSinkDescription: string;
  sounds: SoundFile[];
}

export interface AudioApi {
  getState: () => Promise<AudioState>;
  setMicSource: (name: string) => Promise<AudioState>;
  setMicPassthrough: (enabled: boolean) => Promise<AudioState>;
  setHearClips: (enabled: boolean) => Promise<AudioState>;
  addSounds: () => Promise<AudioState>;
  deleteSound: (url: string) => Promise<AudioState>;
  importSounds: (files: File[]) => Promise<AudioState>;
  relistSounds: () => Promise<AudioState>;
  updateSoundMetadata: (url: string, metadata: SoundMetadata) => Promise<AudioState>;
}

export interface ShortcutApplyResult {
  shortcut: string;
  registered: boolean;
  error?: "invalid" | "taken";
}

export interface ShortcutApi {
  get: () => Promise<string>;
  set: (accelerator: string) => Promise<ShortcutApplyResult>;
}

export interface AppSettings {
  /** When true, closing the main window hides it; when false, the app quits. */
  runInBackground: boolean;
}

export interface AppSettingsApi {
  get: () => Promise<AppSettings>;
  set: (patch: Partial<AppSettings>) => Promise<AppSettings>;
}

export interface IpcRendererApi {
  on: (channel: string, listener: (event: unknown, ...args: unknown[]) => void) => void;
  off: (channel: string, ...args: unknown[]) => void;
  send: (channel: string, ...args: unknown[]) => void;
  invoke: <T = unknown>(channel: string, ...args: unknown[]) => Promise<T>;
}

export interface WindowControlsApi {
  minimize: () => void;
  toggleMaximize: () => void;
  close: () => void;
}

export interface RemoteSound {
  /** Numeric myinstants instant id (the `favorite(...)` argument). */
  id: string;
  /** Title as shown on the myinstants page, decoded + trimmed. */
  title: string;
  /** Page slug (/en/instant/<slug>/). */
  slug: string;
  /** `/media/sounds/<filename>` path on myinstants. */
  path: string;
  /** Bare media filename, e.g. `dry-fart.mp3`. */
  file: string;
  /** Absolute URL the renderer can hand to `new Audio(url)` for previewing. */
  url: string;
}

export interface SearchPage {
  results: RemoteSound[];
  /** The page number that was requested. */
  page: number;
  /** Best-effort "has more" hint (a full-size page assumes more exist). */
  hasMore: boolean;
}

export interface SoundsBrowserApi {
  /** Query myinstants.com for instants matching `query`, page defaults to 1. */
  search: (query: string, page?: number) => Promise<SearchPage>;
  /** Download `sound` into the library, returning the refreshed AudioState. */
  download: (sound: { url: string; file: string; title?: string }) => Promise<AudioState>;
}

export interface OverlayApi {
  hide: () => void;
  pointerReady: () => void;
  selected: (url: string) => void;
  playbackEnded: () => void;
}

declare global {
  interface Window {
    audio?: AudioApi;
    ipcRenderer?: IpcRendererApi;
    overlay?: OverlayApi;
    shortcut?: ShortcutApi;
    appSettings?: AppSettingsApi;
    windowControls?: WindowControlsApi;
    soundsBrowser?: SoundsBrowserApi;
  }
}
