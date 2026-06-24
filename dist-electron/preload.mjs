import { contextBridge, ipcRenderer } from "electron";
//#region electron/preload.ts
contextBridge.exposeInMainWorld("ipcRenderer", {
  on(...args) {
    const [channel, listener] = args;
    return ipcRenderer.on(channel, (event, ...args) => listener(event, ...args));
  },
  off(...args) {
    const [channel, ...omit] = args;
    return ipcRenderer.off(channel, ...omit);
  },
  send(...args) {
    const [channel, ...omit] = args;
    return ipcRenderer.send(channel, ...omit);
  },
  invoke(...args) {
    const [channel, ...omit] = args;
    return ipcRenderer.invoke(channel, ...omit);
  },
});
contextBridge.exposeInMainWorld("audio", {
  getState: () => ipcRenderer.invoke("audio:get-state"),
  setMicSource: (name) => ipcRenderer.invoke("audio:set-mic-source", name),
  setMicPassthrough: (enabled) => ipcRenderer.invoke("audio:set-mic-passthrough", enabled),
  setHearClips: (enabled) => ipcRenderer.invoke("audio:set-hear-clips", enabled),
  addSounds: () => ipcRenderer.invoke("audio:add-sounds"),
  relistSounds: () => ipcRenderer.invoke("audio:relist-sounds"),
});
contextBridge.exposeInMainWorld("overlay", {
  hide: () => ipcRenderer.send("overlay:hide"),
  pointerReady: () => ipcRenderer.send("overlay:pointer-ready"),
  selected: (url) => ipcRenderer.send("overlay:selected", url),
  playbackEnded: () => ipcRenderer.send("overlay:playback-ended"),
});
//#endregion
export {};
