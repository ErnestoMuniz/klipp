import { execFile } from "node:child_process";
import { app, dialog, ipcMain, protocol, shell } from "electron";
import fs from "node:fs/promises";
import { createReadStream, watch, type FSWatcher } from "node:fs";
import path from "node:path";
import { Readable } from "node:stream";
import { MYINSTANTS_USER_AGENT } from "./myinstants";

/** Sound descriptor from myinstants search results, used for one-click import. */
export interface RemoteSoundImport {
  url: string;
  file: string;
  title?: string;
}

/** Returns true if the target path exists (file or directory), false otherwise. */
async function fileExists(target: string): Promise<boolean> {
  try {
    await fs.access(target);
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Persistent settings
// ---------------------------------------------------------------------------

interface StoredSettings {
  micPassthrough: boolean;
  hearClips: boolean;
  currentMicSource: string | null;
}

const SETTINGS_FILE = "settings.json";
const SOUND_METADATA_FILE = "sound-metadata.json";

/** Custom privileged scheme that streams sound files from userData. */
export const SOUND_SCHEME = "klipp-sound";
/** Cosmetically used as the URL host; the filename lives in the path. */
const SOUND_HOST = "sounds";

function settingsPath(): string {
  return path.join(app.getPath("userData"), SETTINGS_FILE);
}

function soundMetadataPath(): string {
  return path.join(app.getPath("userData"), SOUND_METADATA_FILE);
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

export interface SoundMetadata {
  emoji: string;
  displayName: string;
  /** When provided, updates the overlay-availability flag. */
  inOverlay?: boolean;
}

// Metadata is keyed by the bare filename so it stays stable even if the URL
// scheme used to address sounds changes.
type StoredSoundMetadata = Record<string, Partial<SoundMetadata>>;

async function loadSoundMetadata(): Promise<StoredSoundMetadata> {
  let raw: StoredSoundMetadata = {};
  try {
    const file = await fs.readFile(soundMetadataPath(), "utf-8");
    const parsed = JSON.parse(file) as StoredSoundMetadata;
    if (parsed && typeof parsed === "object") raw = parsed;
  } catch {
    return {};
  }

  // Migrate legacy keys ("sounds/<encodeURIComponent(name)>") to bare names.
  let migrated = false;
  const migratedMeta: StoredSoundMetadata = {};
  for (const [key, value] of Object.entries(raw)) {
    if (key.startsWith("sounds/")) {
      try {
        const name = decodeURIComponent(key.slice("sounds/".length));
        if (!(name in migratedMeta)) {
          migratedMeta[name] = value;
          migrated = true;
          continue;
        }
      } catch {
        /* fall through to keep original key */
      }
    }
    migratedMeta[key] = value;
  }
  if (migrated) await saveSoundMetadata(migratedMeta).catch(() => {});
  return migratedMeta;
}

async function saveSoundMetadata(metadata: StoredSoundMetadata): Promise<void> {
  await fs.mkdir(path.dirname(soundMetadataPath()), { recursive: true });
  await fs.writeFile(soundMetadataPath(), JSON.stringify(metadata, null, 2), "utf-8");
}

// Clips-only sink. The renderer plays soundboard clips here (via PULSE_SINK).
// Keeping clips isolated from the mic lets us send ONLY clips to the user's
// speakers without their own voice leaking back.
const CLIPS_SINK_NAME = "klipp-clips";
const CLIPS_SINK_DESCRIPTION = "Klipp-Clips";

// Mixing bus: receives clips (via a loopback) and the real mic (via a
// loopback). Its monitor feeds the virtual microphone, so Discord hears both.
const MIX_SINK_NAME = "klipp-mix";
const MIX_SINK_DESCRIPTION = "Klipp-Mix";

// Virtual microphone exposed to Discord. Created with module-remap-source so it
// is a real Audio/Source (media.class = "Audio/Source") instead of a sink
// monitor, which Discord would classify as an output device. Discord must pick
// this as its INPUT device.
const VIRTUAL_MIC_NAME = "soundboard-mic";
const VIRTUAL_MIC_DESCRIPTION = "Klipp-Mic";

const SOUNDS_SUBDIR = "sounds";
const AUDIO_EXTS = new Set(["mp3", "wav", "ogg", "flac", "m4a", "opus", "webm", "aac"]);

export interface MicSource {
  name: string;
  description: string;
}

export interface SoundFile {
  emoji: string;
  displayName: string;
  name: string;
  url: string;
  /** Whether this sound is selectable from the quick overlay picker. */
  inOverlay: boolean;
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
  // org.freedesktop.Platform includes pactl. Inside Flatpak it talks to the
  // host's PulseAudio/PipeWire server through the exposed PulseAudio socket,
  // so the host does not need to provide the command-line client.
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

function deviceDescription(output: string, deviceName: string, kind: "Source" | "Sink"): string {
  const blocks = output.split(/\n\s*\n/);
  for (const block of blocks) {
    if (!block.startsWith(`${kind} #`)) continue;
    const nameMatch = /\n\tName: (.+)/.exec(block);
    if (nameMatch && nameMatch[1].trim() === deviceName) {
      const descMatch = /\n\tDescription: (.+)/.exec(block);
      return descMatch ? descMatch[1].trim() : deviceName;
    }
  }
  return deviceName;
}

export class AudioManager {
  private loaded: LoadedModule[] = [];
  private existed = new Set<ModuleKind>();
  private soundsWatcher: FSWatcher | null = null;
  private soundsWatchTimer: NodeJS.Timeout | null = null;

  ready = false;
  error: string | null = null;
  micSources: MicSource[] = [];
  currentMicSource: string | null = null;
  micPassthrough = false;
  hearClips = true;
  defaultSink = "";
  defaultSinkDescription = "";
  discordDeviceName = VIRTUAL_MIC_DESCRIPTION;
  sounds: SoundFile[] = [];
  private soundMetadata: StoredSoundMetadata = {};

  private soundsDir(): string {
    // Sounds are user data, not app resources: they must be writable in a
    // packaged build (where the bundle is a read-only asar) and survive app
    // updates. userData is the per-user application data directory.
    return path.join(app.getPath("userData"), SOUNDS_SUBDIR);
  }

  /** URL the renderer uses to play a sound, served by the SOUND_SCHEME handler. */
  private soundUrl(name: string): string {
    return `${SOUND_SCHEME}://${SOUND_HOST}/${encodeURIComponent(name)}`;
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
      defaultSinkDescription: this.defaultSinkDescription,
      sounds: this.sounds,
    };
  }

  async init(): Promise<void> {
    try {
      await this.ensureGraph();
      this.defaultSink = (await runPactl(["get-default-sink"])).trim();
      this.defaultSinkDescription = deviceDescription(
        await runPactl(["list", "sinks"]),
        this.defaultSink,
        "Sink",
      );
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
      this.soundMetadata = await loadSoundMetadata();
      this.sounds = await this.listSounds();
      this.discordDeviceName = deviceDescription(
        await runPactl(["list", "sources"]),
        VIRTUAL_MIC_NAME,
        "Source",
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
        `sink_properties=device.description=${CLIPS_SINK_DESCRIPTION} device.intended_roles=filter`,
      ]);
      this.loaded.push({ kind: "clips-sink", index: idx.trim() });
    }

    if (!mixExisted) {
      const idx = await runPactl([
        "load-module",
        "module-null-sink",
        `sink_name=${MIX_SINK_NAME}`,
        `sink_properties=device.description=${MIX_SINK_DESCRIPTION} device.intended_roles=filter`,
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
        .map((name) => {
          const url = this.soundUrl(name);
          const metadata = this.soundMetadata[name] ?? {};
          const displayName =
            typeof metadata.displayName === "string" && metadata.displayName.trim()
              ? metadata.displayName.trim()
              : path.basename(name, path.extname(name));
          const emoji =
            typeof metadata.emoji === "string" && metadata.emoji.trim()
              ? metadata.emoji.trim()
              : "♪";
          const inOverlay = metadata.inOverlay !== false;
          return { emoji, displayName, name, url, inOverlay };
        });
    } catch {
      return [];
    }
  }

  async relistSounds(): Promise<AudioState> {
    this.sounds = await this.listSounds();
    return this.getState();
  }

  async openSoundsFolder(): Promise<void> {
    const dir = this.soundsDir();
    await fs.mkdir(dir, { recursive: true });
    const error = await shell.openPath(dir);
    if (error) throw new Error(error);
  }

  async watchSounds(onChange: (state: AudioState) => void): Promise<void> {
    const dir = this.soundsDir();
    await fs.mkdir(dir, { recursive: true });
    this.soundsWatcher?.close();
    this.soundsWatcher = watch(dir, { persistent: false }, () => {
      if (this.soundsWatchTimer) clearTimeout(this.soundsWatchTimer);
      this.soundsWatchTimer = setTimeout(() => {
        this.soundsWatchTimer = null;
        void this.relistSounds()
          .then(onChange)
          .catch(() => {});
      }, 100);
    });
    this.soundsWatcher.on("error", () => {
      this.soundsWatcher?.close();
      this.soundsWatcher = null;
    });
  }

  async updateSoundMetadata(url: string, metadata: SoundMetadata): Promise<AudioState> {
    const sound = this.sounds.find((candidate) => candidate.url === url);
    if (!sound) return this.getState();

    const newDisplayName =
      typeof metadata.displayName === "string" ? metadata.displayName.trim() : "";
    const newEmoji = typeof metadata.emoji === "string" ? metadata.emoji.trim() : "";
    const displayName = newDisplayName || sound.displayName;
    const emoji = newEmoji || sound.emoji;
    const inOverlay = metadata.inOverlay ?? sound.inOverlay;
    this.soundMetadata = {
      ...this.soundMetadata,
      [sound.name]: { displayName, emoji, inOverlay },
    };
    await saveSoundMetadata(this.soundMetadata);
    this.sounds = await this.listSounds();
    return this.getState();
  }

  async deleteSound(url: string): Promise<AudioState> {
    const sound = this.sounds.find((candidate) => candidate.url === url);
    if (!sound) return this.getState();

    await fs.unlink(path.join(this.soundsDir(), sound.name));
    const { [sound.name]: _removed, ...remainingMetadata } = this.soundMetadata;
    this.soundMetadata = remainingMetadata;
    await saveSoundMetadata(this.soundMetadata);
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
    return this.importSounds(res.filePaths);
  }

  async importSounds(files: string[]): Promise<AudioState> {
    const validFiles: string[] = [];
    for (const file of files) {
      if (typeof file !== "string" || !AUDIO_EXTS.has(path.extname(file).slice(1).toLowerCase())) {
        continue;
      }
      const stats = await fs.stat(file).catch(() => null);
      if (stats?.isFile()) validFiles.push(file);
    }
    if (validFiles.length === 0) return this.getState();

    const dir = this.soundsDir();
    await fs.mkdir(dir, { recursive: true });
    for (const file of validFiles) {
      await fs.copyFile(file, path.join(dir, path.basename(file)));
    }
    this.sounds = await this.listSounds();
    return this.getState();
  }

  /**
   * Import a sound from myinstants (or any URL): download its bytes into the
   * userData sounds directory, persist a friendly display name from the
   * search result, then refresh the loaded sounds list.
   *
   * @returns the refreshed AudioState (no-op shape if anything went wrong).
   */
  async installRemoteSound(sound: RemoteSoundImport): Promise<AudioState> {
    // Validate the requested filename before touching the filesystem.
    const file = sound.file?.trim();
    if (!file || file.includes("/") || file.includes("\\")) {
      throw new Error("Invalid sound filename");
    }
    if (!sound.url) throw new Error("Missing sound URL");

    const dir = this.soundsDir();
    await fs.mkdir(dir, { recursive: true });

    // If the extension isn't a known audio type, fall back to .mp3 (myinstants
    // media is mp3 anyway, but a stray .txt URL shouldn't crash listSounds).
    const ext = path.extname(file).slice(1).toLowerCase();
    const candidate = ext && AUDIO_EXTS.has(ext) ? file : `${file}.mp3`;

    // Pick a filename that doesn't collide with an existing one.
    let unique = candidate;
    let attempt = 1;
    const known = new Set(this.sounds.map((entry) => entry.name));
    while (known.has(unique) || (await fileExists(path.join(dir, unique)))) {
      const candidateExt = path.extname(candidate);
      const candidateBase = path.basename(candidate, candidateExt);
      unique = `${candidateBase}-${attempt}${candidateExt}`;
      attempt += 1;
    }

    // Download bytes. A 4xx/5xx surfaces as a thrown Error to the renderer.
    const response = await fetch(sound.url, {
      headers: { "User-Agent": MYINSTANTS_USER_AGENT, Accept: "audio/*,*/*;q=0.1" },
      redirect: "follow",
    });
    if (!response.ok) throw new Error(`Download failed (status ${response.status})`);
    const body = await response.arrayBuffer();
    if (body.byteLength === 0) throw new Error("Downloaded file is empty");
    await fs.writeFile(path.join(dir, unique), Buffer.from(body));

    const title =
      typeof sound.title === "string" && sound.title.trim()
        ? sound.title.trim()
        : path.basename(unique, path.extname(unique));
    this.soundMetadata = {
      ...this.soundMetadata,
      [unique]: { displayName: title, emoji: "♪", inOverlay: true },
    };
    await saveSoundMetadata(this.soundMetadata);
    this.sounds = await this.listSounds();
    return this.getState();
  }

  /** Sounds the quick overlay picker may show (those opted into the overlay). */
  overlaySounds(): SoundFile[] {
    return this.sounds.filter((sound) => sound.inOverlay);
  }

  async cleanup(): Promise<void> {
    if (this.soundsWatchTimer) clearTimeout(this.soundsWatchTimer);
    this.soundsWatchTimer = null;
    this.soundsWatcher?.close();
    this.soundsWatcher = null;
    for (const mod of [...this.loaded].reverse()) {
      if (this.existed.has(mod.kind)) continue;
      await runPactl(["unload-module", mod.index]).catch(() => {});
    }
    this.loaded = [];
  }
}

const MIME_BY_EXT: Record<string, string> = {
  mp3: "audio/mpeg",
  wav: "audio/wav",
  ogg: "audio/ogg",
  flac: "audio/flac",
  m4a: "audio/mp4",
  opus: "audio/ogg",
  webm: "audio/webm",
  aac: "audio/aac",
};

/**
 * Serves sound files from the userData sounds directory to the renderer via
 * the `klipp-sound://` scheme. Works identically in dev and packaged builds,
 * supports `new Audio(url)`, and honors Range requests for seeking.
 *
 * Must be called from `registerSchemesAsPrivileged` (in main, before app ready)
 * for the scheme to be usable as a secure, fetch-capable, streaming scheme.
 */
export function registerSoundProtocol(): void {
  const dir = path.join(app.getPath("userData"), SOUNDS_SUBDIR);

  protocol.handle(SOUND_SCHEME, async (request) => {
    const parsed = new URL(request.url);
    const name = decodeURIComponent(parsed.pathname.slice(1));
    if (!name || name.includes("/") || name.includes("\\")) {
      return new Response("Not found", { status: 404 });
    }

    const filePath = path.join(dir, name);
    // Guard against traversal: the resolved path must stay inside `dir`.
    const relative = path.relative(dir, filePath);
    if (relative.startsWith("..") || path.isAbsolute(relative)) {
      return new Response("Not found", { status: 404 });
    }

    let stats;
    try {
      stats = await fs.stat(filePath);
    } catch {
      return new Response("Not found", { status: 404 });
    }
    if (!stats.isFile()) return new Response("Not found", { status: 404 });

    const mime =
      MIME_BY_EXT[path.extname(name).slice(1).toLowerCase()] ?? "application/octet-stream";

    const range = request.headers.get("range");
    if (range) {
      const match = /^bytes=(\d+)-(\d*)$/.exec(range);
      if (match) {
        const start = Number.parseInt(match[1], 10);
        const end = match[2] ? Number.parseInt(match[2], 10) : stats.size - 1;
        if (Number.isNaN(start) || Number.isNaN(end) || start > end || end >= stats.size) {
          return new Response("Range Not Satisfiable", {
            status: 416,
            headers: { "content-range": `bytes */${stats.size}` },
          });
        }
        const body = Readable.toWeb(createReadStream(filePath, { start, end })) as ReadableStream;
        return new Response(body, {
          status: 206,
          headers: {
            "content-type": mime,
            "content-length": String(end - start + 1),
            "content-range": `bytes ${start}-${end}/${stats.size}`,
            "accept-ranges": "bytes",
          },
        });
      }
    }

    const body = Readable.toWeb(createReadStream(filePath)) as ReadableStream;
    return new Response(body, {
      status: 200,
      headers: {
        "content-type": mime,
        "content-length": String(stats.size),
        "accept-ranges": "bytes",
      },
    });
  });
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
  ipcMain.handle("audio:open-sounds-folder", async () => manager.openSoundsFolder());
  ipcMain.handle("audio:delete-sound", async (_event, url: string) => manager.deleteSound(url));
  ipcMain.handle("audio:import-sounds", async (_event, files: string[]) =>
    manager.importSounds(files),
  );
  ipcMain.handle("audio:relist-sounds", async () => manager.relistSounds());
  ipcMain.handle("audio:install-remote-sound", async (_event, sound: RemoteSoundImport) =>
    manager.installRemoteSound(sound),
  );
  ipcMain.handle(
    "audio:update-sound-metadata",
    async (_event, url: string, metadata: SoundMetadata) =>
      manager.updateSoundMetadata(url, metadata),
  );
}
