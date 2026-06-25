import { execFile } from "node:child_process";
import { app, dialog, ipcMain } from "electron";
import fs from "node:fs/promises";
import path from "node:path";

// ---------------------------------------------------------------------------
// Persistent settings
// ---------------------------------------------------------------------------

interface StoredSettings {
  micPassthrough: boolean;
  hearClips: boolean;
  currentMicSource: string | null;
}

const SETTINGS_FILE = "settings.json";

function settingsPath(): string {
  return path.join(app.getPath("userData"), SETTINGS_FILE);
}

async function loadSettings(): Promise<StoredSettings> {
  const defaults: StoredSettings = {
    micPassthrough: false,
    hearClips: true,
    currentMicSource: null,
  };
  try {
    const raw = await fs.readFile(settingsPath(), "utf-8");
    const parsed = JSON.parse(raw) as Partial<StoredSettings>;
    return { ...defaults, ...parsed };
  } catch {
    return defaults;
  }
}

async function saveSettings(settings: StoredSettings): Promise<void> {
  await fs.mkdir(path.dirname(settingsPath()), { recursive: true });
  await fs.writeFile(settingsPath(), JSON.stringify(settings, null, 2), "utf-8");
}

// Clips-only sink. The renderer plays soundboard clips here (via PULSE_SINK).
// Keeping clips isolated from the mic lets us send ONLY clips to the user's
// speakers without their own voice leaking back.
const CLIPS_SINK_NAME = "soundboard-clips";
const CLIPS_SINK_DESCRIPTION = "Soundboard-Clips";

// Mixing bus: receives clips (via a loopback) and the real mic (via a
// loopback). Its monitor feeds the virtual microphone, so Discord hears both.
const MIX_SINK_NAME = "soundboard-mix";
const MIX_SINK_DESCRIPTION = "Soundboard-Mix";

// Virtual microphone exposed to Discord. Created with module-remap-source so it
// is a real Audio/Source (media.class = "Audio/Source") instead of a sink
// monitor, which Discord would classify as an output device. Discord must pick
// this as its INPUT device.
const VIRTUAL_MIC_NAME = "soundboard-mic";
const VIRTUAL_MIC_DESCRIPTION = "Soundboard-Mic";

const SOUNDS_SUBDIR = "sounds";
const AUDIO_EXTS = new Set(["mp3", "wav", "ogg", "flac", "m4a", "opus", "webm", "aac"]);

export interface MicSource {
  name: string;
  description: string;
}

export interface SoundFile {
  name: string;
  url: string;
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

type ModuleKind =
  | "clips-sink"
  | "mix-sink"
  | "remap"
  | "clips-to-mix"
  | "mic-loopback"
  | "hear-clips-loopback";

interface LoadedModule {
  kind: ModuleKind;
  index: string;
}

function runPactl(args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile("pactl", args, { timeout: 5000 }, (err, stdout, stderr) => {
      if (err) {
        reject(new Error(stderr.trim() || err.message));
        return;
      }
      resolve(stdout);
    });
  });
}

function sourceExists(output: string, name: string): boolean {
  return output.split("\n").some((line) => line.split("\t")[1] === name);
}

function sinkExists(output: string, name: string): boolean {
  return sourceExists(output, name);
}

function parseSources(output: string): MicSource[] {
  const blocks = output.split(/\n\s*\n/);
  const result: MicSource[] = [];
  for (const block of blocks) {
    if (!block.startsWith("Source #")) continue;
    const nameMatch = /\n\tName: (.+)/.exec(block);
    if (!nameMatch) continue;
    const name = nameMatch[1].trim();
    if (name.endsWith(".monitor")) continue;
    if (name === VIRTUAL_MIC_NAME) continue;
    const descMatch = /\n\tDescription: (.+)/.exec(block);
    result.push({ name, description: descMatch ? descMatch[1].trim() : name });
  }
  return result;
}

function sourceDescription(output: string, sourceName: string): string {
  const blocks = output.split(/\n\s*\n/);
  for (const block of blocks) {
    if (!block.startsWith("Source #")) continue;
    const nameMatch = /\n\tName: (.+)/.exec(block);
    if (nameMatch && nameMatch[1].trim() === sourceName) {
      const descMatch = /\n\tDescription: (.+)/.exec(block);
      return descMatch ? descMatch[1].trim() : sourceName;
    }
  }
  return sourceName;
}

export class AudioManager {
  private loaded: LoadedModule[] = [];
  private existed = new Set<ModuleKind>();

  ready = false;
  error: string | null = null;
  micSources: MicSource[] = [];
  currentMicSource: string | null = null;
  micPassthrough = false;
  hearClips = true;
  defaultSink = "";
  discordDeviceName = VIRTUAL_MIC_DESCRIPTION;
  sounds: SoundFile[] = [];

  private soundsDir(): string {
    const root = process.env.APP_ROOT ?? process.cwd();
    return path.join(root, "public", SOUNDS_SUBDIR);
  }

  getState(): AudioState {
    return {
      ready: this.ready,
      error: this.error,
      virtualMic: VIRTUAL_MIC_NAME,
      discordDeviceName: this.discordDeviceName,
      clipsSink: CLIPS_SINK_NAME,
      mixSink: MIX_SINK_NAME,
      micSources: this.micSources,
      currentMicSource: this.currentMicSource,
      micPassthrough: this.micPassthrough,
      hearClips: this.hearClips,
      defaultSink: this.defaultSink,
      sounds: this.sounds,
    };
  }

  async init(): Promise<void> {
    try {
      await this.ensureGraph();
      this.defaultSink = (await runPactl(["get-default-sink"])).trim();
      const defaultSource = (await runPactl(["get-default-source"])).trim();
      this.micSources = await this.listMicSources();

      // Load persisted settings and validate that the saved mic source still exists.
      const saved = await loadSettings();
      const savedMicValid =
        saved.currentMicSource != null &&
        this.micSources.some((m) => m.name === saved.currentMicSource);
      this.currentMicSource = savedMicValid
        ? saved.currentMicSource
        : this.micSources.some((m) => m.name === defaultSource)
          ? defaultSource
          : (this.micSources[0]?.name ?? null);
      this.micPassthrough = savedMicValid ? saved.micPassthrough : this.currentMicSource !== null;
      this.hearClips = saved.hearClips;

      await this.applyMicLoopback();
      await this.applyHearClips();
      this.sounds = await this.listSounds();
      this.discordDeviceName = sourceDescription(
        await runPactl(["list", "sources"]),
        VIRTUAL_MIC_NAME,
      );
      this.ready = true;
      this.error = null;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      this.ready = false;
    }
  }

  private async ensureGraph(): Promise<void> {
    const sinks = await runPactl(["list", "short", "sinks"]);
    const sources = await runPactl(["list", "short", "sources"]);

    const clipsExisted = sinkExists(sinks, CLIPS_SINK_NAME);
    const mixExisted = sinkExists(sinks, MIX_SINK_NAME);
    const remapExisted = sourceExists(sources, VIRTUAL_MIC_NAME);
    if (clipsExisted) this.existed.add("clips-sink");
    if (mixExisted) this.existed.add("mix-sink");
    if (remapExisted) this.existed.add("remap");

    if (!clipsExisted) {
      const idx = await runPactl([
        "load-module",
        "module-null-sink",
        `sink_name=${CLIPS_SINK_NAME}`,
        `sink_properties=device.description=${CLIPS_SINK_DESCRIPTION}`,
      ]);
      this.loaded.push({ kind: "clips-sink", index: idx.trim() });
    }

    if (!mixExisted) {
      const idx = await runPactl([
        "load-module",
        "module-null-sink",
        `sink_name=${MIX_SINK_NAME}`,
        `sink_properties=device.description=${MIX_SINK_DESCRIPTION}`,
      ]);
      this.loaded.push({ kind: "mix-sink", index: idx.trim() });
    }

    if (!remapExisted) {
      const idx = await runPactl([
        "load-module",
        "module-remap-source",
        `source_name=${VIRTUAL_MIC_NAME}`,
        `master=${MIX_SINK_NAME}.monitor`,
        "channels=2",
        "channel_map=front-left,front-right",
        `source_properties=device.description=${VIRTUAL_MIC_DESCRIPTION}`,
      ]);
      this.loaded.push({ kind: "remap", index: idx.trim() });
    }

    // Always route clips into the mix so Discord hears them.
    const idx = await runPactl([
      "load-module",
      "module-loopback",
      `source=${CLIPS_SINK_NAME}.monitor`,
      `sink=${MIX_SINK_NAME}`,
    ]);
    this.loaded.push({ kind: "clips-to-mix", index: idx.trim() });
  }

  private async listMicSources(): Promise<MicSource[]> {
    return parseSources(await runPactl(["list", "sources"]));
  }

  private async unloadByKind(kind: ModuleKind): Promise<void> {
    const keep: LoadedModule[] = [];
    for (const mod of this.loaded) {
      if (mod.kind === kind) {
        await runPactl(["unload-module", mod.index]).catch(() => {});
      } else {
        keep.push(mod);
      }
    }
    this.loaded = keep;
  }

  private async applyMicLoopback(): Promise<void> {
    await this.unloadByKind("mic-loopback");
    if (!this.micPassthrough || !this.currentMicSource) return;
    const idx = await runPactl([
      "load-module",
      "module-loopback",
      `source=${this.currentMicSource}`,
      `sink=${MIX_SINK_NAME}`,
    ]);
    this.loaded.push({ kind: "mic-loopback", index: idx.trim() });
  }

  private async applyHearClips(): Promise<void> {
    await this.unloadByKind("hear-clips-loopback");
    if (!this.hearClips || !this.defaultSink) return;
    const idx = await runPactl([
      "load-module",
      "module-loopback",
      `source=${CLIPS_SINK_NAME}.monitor`,
      `sink=${this.defaultSink}`,
    ]);
    this.loaded.push({ kind: "hear-clips-loopback", index: idx.trim() });
  }

  async setMicSource(name: string): Promise<AudioState> {
    this.currentMicSource = name;
    await this.applyMicLoopback();
    await this.persistSettings();
    return this.getState();
  }

  async setMicPassthrough(enabled: boolean): Promise<AudioState> {
    this.micPassthrough = enabled;
    await this.applyMicLoopback();
    await this.persistSettings();
    return this.getState();
  }

  async setHearClips(enabled: boolean): Promise<AudioState> {
    this.hearClips = enabled;
    await this.applyHearClips();
    await this.persistSettings();
    return this.getState();
  }

  private async persistSettings(): Promise<void> {
    await saveSettings({
      micPassthrough: this.micPassthrough,
      hearClips: this.hearClips,
      currentMicSource: this.currentMicSource,
    }).catch(() => {});
  }

  private async listSounds(): Promise<SoundFile[]> {
    const dir = this.soundsDir();
    try {
      await fs.mkdir(dir, { recursive: true });
      const entries = await fs.readdir(dir);
      return entries
        .filter((name) => AUDIO_EXTS.has(path.extname(name).slice(1).toLowerCase()))
        .sort((a, b) => a.localeCompare(b))
        .map((name) => ({ name, url: `sounds/${encodeURIComponent(name)}` }));
    } catch {
      return [];
    }
  }

  async relistSounds(): Promise<AudioState> {
    this.sounds = await this.listSounds();
    return this.getState();
  }

  async addSounds(): Promise<AudioState> {
    const res = await dialog.showOpenDialog({
      title: "Selecionar áudios",
      filters: [{ name: "Áudio", extensions: [...AUDIO_EXTS] }],
      properties: ["openFile", "multiSelections"],
    });
    if (res.canceled || res.filePaths.length === 0) {
      return this.getState();
    }
    const dir = this.soundsDir();
    await fs.mkdir(dir, { recursive: true });
    for (const file of res.filePaths) {
      await fs.copyFile(file, path.join(dir, path.basename(file)));
    }
    this.sounds = await this.listSounds();
    return this.getState();
  }

  async cleanup(): Promise<void> {
    for (const mod of [...this.loaded].reverse()) {
      if (this.existed.has(mod.kind)) continue;
      await runPactl(["unload-module", mod.index]).catch(() => {});
    }
    this.loaded = [];
  }
}

export function registerAudioIpc(manager: AudioManager): void {
  ipcMain.handle("audio:get-state", async () => manager.getState());
  ipcMain.handle("audio:set-mic-source", async (_event, name: string) =>
    manager.setMicSource(name),
  );
  ipcMain.handle("audio:set-mic-passthrough", async (_event, enabled: boolean) =>
    manager.setMicPassthrough(enabled),
  );
  ipcMain.handle("audio:set-hear-clips", async (_event, enabled: boolean) =>
    manager.setHearClips(enabled),
  );
  ipcMain.handle("audio:add-sounds", async () => manager.addSounds());
  ipcMain.handle("audio:relist-sounds", async () => manager.relistSounds());
}
