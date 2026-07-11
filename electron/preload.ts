import { ipcRenderer, contextBridge, webUtils } from "electron";
import type { SoundMetadata } from "./audio";

contextBridge.exposeInMainWorld("ipcRenderer", {
  on(...args: Parameters<typeof ipcRenderer.on>) {
    const [channel, listener] = args;
    return ipcRenderer.on(channel, (event, ...args) => listener(event, ...args));
  },
  off(...args: Parameters<typeof ipcRenderer.off>) {
    const [channel, ...omit] = args;
    return ipcRenderer.off(channel, ...omit);
  },
  send(...args: Parameters<typeof ipcRenderer.send>) {
    const [channel, ...omit] = args;
    return ipcRenderer.send(channel, ...omit);
  },
  invoke(...args: Parameters<typeof ipcRenderer.invoke>) {
    const [channel, ...omit] = args;
    return ipcRenderer.invoke(channel, ...omit);
  },
});

contextBridge.exposeInMainWorld("audio", {
  getState: () => ipcRenderer.invoke("audio:get-state"),
  setMicSource: (name: string) => ipcRenderer.invoke("audio:set-mic-source", name),
  setMicPassthrough: (enabled: boolean) => ipcRenderer.invoke("audio:set-mic-passthrough", enabled),
  setHearClips: (enabled: boolean) => ipcRenderer.invoke("audio:set-hear-clips", enabled),
  addSounds: () => ipcRenderer.invoke("audio:add-sounds"),
  deleteSound: (url: string) => ipcRenderer.invoke("audio:delete-sound", url),
  importSounds: (files: File[]) =>
    ipcRenderer.invoke(
      "audio:import-sounds",
      files.map((file) => webUtils.getPathForFile(file)),
    ),
  relistSounds: () => ipcRenderer.invoke("audio:relist-sounds"),
  updateSoundMetadata: (url: string, metadata: SoundMetadata) =>
    ipcRenderer.invoke("audio:update-sound-metadata", url, metadata),
});

contextBridge.exposeInMainWorld("shortcut", {
  get: () => ipcRenderer.invoke("shortcut:get"),
  set: (accelerator: string) => ipcRenderer.invoke("shortcut:set", accelerator),
});

contextBridge.exposeInMainWorld("appSettings", {
  get: () => ipcRenderer.invoke("app-settings:get"),
  set: (patch: Record<string, unknown>) => ipcRenderer.invoke("app-settings:set", patch),
});

contextBridge.exposeInMainWorld("windowControls", {
  minimize: () => ipcRenderer.send("window:minimize"),
  toggleMaximize: () => ipcRenderer.send("window:toggle-maximize"),
  close: () => ipcRenderer.send("window:close"),
});

contextBridge.exposeInMainWorld("soundsBrowser", {
  search: (query: string, page?: number) => ipcRenderer.invoke("myinstants:search", query, page),
  download: (sound: { url: string; file: string; title?: string }) =>
    ipcRenderer.invoke("audio:install-remote-sound", sound),
});

contextBridge.exposeInMainWorld("overlay", {
  hide: () => ipcRenderer.send("overlay:hide"),
  pointerReady: () => ipcRenderer.send("overlay:pointer-ready"),
  selected: (url: string) => ipcRenderer.send("overlay:selected", url),
  playbackEnded: () => ipcRenderer.send("overlay:playback-ended"),
});
