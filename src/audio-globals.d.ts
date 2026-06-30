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
  sounds: SoundFile[];
}

export interface AudioApi {
  getState: () => Promise<AudioState>;
  setMicSource: (name: string) => Promise<AudioState>;
  setMicPassthrough: (enabled: boolean) => Promise<AudioState>;
  setHearClips: (enabled: boolean) => Promise<AudioState>;
  addSounds: () => Promise<AudioState>;
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
    windowControls?: WindowControlsApi;
  }
}
