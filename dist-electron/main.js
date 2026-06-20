import { BrowserWindow, Menu, Tray, app, dialog, globalShortcut, ipcMain, nativeImage, screen } from "electron";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFile } from "node:child_process";
import fs from "node:fs/promises";
//#region electron/audio.ts
var CLIPS_SINK_NAME = "soundboard-clips";
var CLIPS_SINK_DESCRIPTION = "Soundboard-Clips";
var MIX_SINK_NAME = "soundboard-mix";
var MIX_SINK_DESCRIPTION = "Soundboard-Mix";
var VIRTUAL_MIC_NAME = "soundboard-mic";
var VIRTUAL_MIC_DESCRIPTION = "Soundboard-Mic";
var SOUNDS_SUBDIR = "sounds";
var AUDIO_EXTS = new Set([
	"mp3",
	"wav",
	"ogg",
	"flac",
	"m4a",
	"opus",
	"webm",
	"aac"
]);
function runPactl(args) {
	return new Promise((resolve, reject) => {
		execFile("pactl", args, { timeout: 5e3 }, (err, stdout, stderr) => {
			if (err) {
				reject(new Error(stderr.trim() || err.message));
				return;
			}
			resolve(stdout);
		});
	});
}
function sourceExists(output, name) {
	return output.split("\n").some((line) => line.split("	")[1] === name);
}
function sinkExists(output, name) {
	return sourceExists(output, name);
}
function parseSources(output) {
	const blocks = output.split(/\n\s*\n/);
	const result = [];
	for (const block of blocks) {
		if (!block.startsWith("Source #")) continue;
		const nameMatch = /\n\tName: (.+)/.exec(block);
		if (!nameMatch) continue;
		const name = nameMatch[1].trim();
		if (name.endsWith(".monitor")) continue;
		if (name === VIRTUAL_MIC_NAME) continue;
		const descMatch = /\n\tDescription: (.+)/.exec(block);
		result.push({
			name,
			description: descMatch ? descMatch[1].trim() : name
		});
	}
	return result;
}
function sourceDescription(output, sourceName) {
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
var AudioManager = class {
	constructor() {
		this.loaded = [];
		this.existed = /* @__PURE__ */ new Set();
		this.ready = false;
		this.error = null;
		this.micSources = [];
		this.currentMicSource = null;
		this.micPassthrough = false;
		this.hearClips = true;
		this.defaultSink = "";
		this.discordDeviceName = VIRTUAL_MIC_DESCRIPTION;
		this.sounds = [];
	}
	soundsDir() {
		const root = process.env.APP_ROOT ?? process.cwd();
		return path.join(root, "public", SOUNDS_SUBDIR);
	}
	getState() {
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
			sounds: this.sounds
		};
	}
	async init() {
		try {
			await this.ensureGraph();
			this.defaultSink = (await runPactl(["get-default-sink"])).trim();
			const defaultSource = (await runPactl(["get-default-source"])).trim();
			this.micSources = await this.listMicSources();
			this.currentMicSource = this.micSources.some((m) => m.name === defaultSource) ? defaultSource : this.micSources[0]?.name ?? null;
			this.micPassthrough = this.currentMicSource !== null;
			await this.applyMicLoopback();
			await this.applyHearClips();
			this.sounds = await this.listSounds();
			this.discordDeviceName = sourceDescription(await runPactl(["list", "sources"]), VIRTUAL_MIC_NAME);
			this.ready = true;
			this.error = null;
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			this.ready = false;
		}
	}
	async ensureGraph() {
		const sinks = await runPactl([
			"list",
			"short",
			"sinks"
		]);
		const sources = await runPactl([
			"list",
			"short",
			"sources"
		]);
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
				`sink_properties=device.description=${CLIPS_SINK_DESCRIPTION}`
			]);
			this.loaded.push({
				kind: "clips-sink",
				index: idx.trim()
			});
		}
		if (!mixExisted) {
			const idx = await runPactl([
				"load-module",
				"module-null-sink",
				`sink_name=${MIX_SINK_NAME}`,
				`sink_properties=device.description=${MIX_SINK_DESCRIPTION}`
			]);
			this.loaded.push({
				kind: "mix-sink",
				index: idx.trim()
			});
		}
		if (!remapExisted) {
			const idx = await runPactl([
				"load-module",
				"module-remap-source",
				`source_name=${VIRTUAL_MIC_NAME}`,
				`master=${MIX_SINK_NAME}.monitor`,
				"channels=2",
				"channel_map=front-left,front-right",
				`source_properties=device.description=${VIRTUAL_MIC_DESCRIPTION}`
			]);
			this.loaded.push({
				kind: "remap",
				index: idx.trim()
			});
		}
		const idx = await runPactl([
			"load-module",
			"module-loopback",
			`source=${CLIPS_SINK_NAME}.monitor`,
			`sink=${MIX_SINK_NAME}`
		]);
		this.loaded.push({
			kind: "clips-to-mix",
			index: idx.trim()
		});
	}
	async listMicSources() {
		return parseSources(await runPactl(["list", "sources"]));
	}
	async unloadByKind(kind) {
		const keep = [];
		for (const mod of this.loaded) if (mod.kind === kind) await runPactl(["unload-module", mod.index]).catch(() => {});
		else keep.push(mod);
		this.loaded = keep;
	}
	async applyMicLoopback() {
		await this.unloadByKind("mic-loopback");
		if (!this.micPassthrough || !this.currentMicSource) return;
		const idx = await runPactl([
			"load-module",
			"module-loopback",
			`source=${this.currentMicSource}`,
			`sink=${MIX_SINK_NAME}`
		]);
		this.loaded.push({
			kind: "mic-loopback",
			index: idx.trim()
		});
	}
	async applyHearClips() {
		await this.unloadByKind("hear-clips-loopback");
		if (!this.hearClips || !this.defaultSink) return;
		const idx = await runPactl([
			"load-module",
			"module-loopback",
			`source=${CLIPS_SINK_NAME}.monitor`,
			`sink=${this.defaultSink}`
		]);
		this.loaded.push({
			kind: "hear-clips-loopback",
			index: idx.trim()
		});
	}
	async setMicSource(name) {
		this.currentMicSource = name;
		await this.applyMicLoopback();
		return this.getState();
	}
	async setMicPassthrough(enabled) {
		this.micPassthrough = enabled;
		await this.applyMicLoopback();
		return this.getState();
	}
	async setHearClips(enabled) {
		this.hearClips = enabled;
		await this.applyHearClips();
		return this.getState();
	}
	async listSounds() {
		const dir = this.soundsDir();
		try {
			await fs.mkdir(dir, { recursive: true });
			return (await fs.readdir(dir)).filter((name) => AUDIO_EXTS.has(path.extname(name).slice(1).toLowerCase())).sort((a, b) => a.localeCompare(b)).map((name) => ({
				name,
				url: `sounds/${encodeURIComponent(name)}`
			}));
		} catch {
			return [];
		}
	}
	async relistSounds() {
		this.sounds = await this.listSounds();
		return this.getState();
	}
	async addSounds() {
		const res = await dialog.showOpenDialog({
			title: "Selecionar áudios",
			filters: [{
				name: "Áudio",
				extensions: [...AUDIO_EXTS]
			}],
			properties: ["openFile", "multiSelections"]
		});
		if (res.canceled || res.filePaths.length === 0) return this.getState();
		const dir = this.soundsDir();
		await fs.mkdir(dir, { recursive: true });
		for (const file of res.filePaths) await fs.copyFile(file, path.join(dir, path.basename(file)));
		this.sounds = await this.listSounds();
		return this.getState();
	}
	async cleanup() {
		for (const mod of [...this.loaded].reverse()) {
			if (this.existed.has(mod.kind)) continue;
			await runPactl(["unload-module", mod.index]).catch(() => {});
		}
		this.loaded = [];
	}
};
function registerAudioIpc(manager) {
	ipcMain.handle("audio:get-state", async () => manager.getState());
	ipcMain.handle("audio:set-mic-source", async (_event, name) => manager.setMicSource(name));
	ipcMain.handle("audio:set-mic-passthrough", async (_event, enabled) => manager.setMicPassthrough(enabled));
	ipcMain.handle("audio:set-hear-clips", async (_event, enabled) => manager.setHearClips(enabled));
	ipcMain.handle("audio:add-sounds", async () => manager.addSounds());
	ipcMain.handle("audio:relist-sounds", async () => manager.relistSounds());
}
//#endregion
//#region electron/main.ts
var __dirname = path.dirname(fileURLToPath(import.meta.url));
process.env.APP_ROOT = path.join(__dirname, "..");
var VITE_DEV_SERVER_URL = process.env["VITE_DEV_SERVER_URL"];
var RENDERER_DIST = path.join(process.env.APP_ROOT, "dist");
process.env.VITE_PUBLIC = VITE_DEV_SERVER_URL ? path.join(process.env.APP_ROOT, "public") : RENDERER_DIST;
process.env.PULSE_SINK = "soundboard-clips";
var GLOBAL_SHORTCUT = "Alt+Shift+S";
var OVERLAY_TIMEOUT_MS = 15e3;
var audio = new AudioManager();
var win = null;
var tray = null;
var isQuitting = false;
var shortcutRegistered = false;
var overlayHideTimer = null;
var overlayActive = false;
var overlays = /* @__PURE__ */ new Map();
function loadRenderer(window, query) {
	if (VITE_DEV_SERVER_URL) {
		const url = new URL(VITE_DEV_SERVER_URL);
		for (const [key, value] of Object.entries(query ?? {})) url.searchParams.set(key, value);
		window.loadURL(url.toString());
	} else window.loadFile(path.join(RENDERER_DIST, "index.html"), { query });
}
function createWindow() {
	win = new BrowserWindow({
		icon: path.join(process.env.VITE_PUBLIC, "favicon.svg"),
		width: 900,
		height: 720,
		webPreferences: {
			preload: path.join(__dirname, "preload.mjs"),
			sandbox: false,
			backgroundThrottling: false
		}
	});
	win.on("close", (event) => {
		if (!isQuitting) {
			event.preventDefault();
			win?.hide();
		}
	});
	win.webContents.on("did-finish-load", () => {
		win?.webContents.send("main-process-message", (/* @__PURE__ */ new Date()).toLocaleString());
	});
	loadRenderer(win);
}
function showWindow() {
	if (win) {
		if (win.isMinimized()) win.restore();
		win.show();
		win.focus();
	} else createWindow();
}
function createTray() {
	const iconPath = path.join(process.env.VITE_PUBLIC, "tray-icon-32.png");
	const icon = nativeImage.createFromPath(iconPath);
	tray = new Tray(icon.isEmpty() ? nativeImage.createEmpty() : icon);
	const menu = Menu.buildFromTemplate([
		{
			label: "Soundboard",
			enabled: false
		},
		{ type: "separator" },
		{
			label: "Mostrar",
			click: () => showWindow()
		},
		{
			label: "Sair",
			click: () => quitApp()
		}
	]);
	tray.setContextMenu(menu);
	tray.setToolTip("Soundboard — clique para abrir");
	tray.on("click", () => showWindow());
}
function quitApp() {
	isQuitting = true;
	app.quit();
}
function clearOverlayTimer() {
	if (overlayHideTimer) {
		clearTimeout(overlayHideTimer);
		overlayHideTimer = null;
	}
}
function hideOverlays() {
	clearOverlayTimer();
	overlayActive = false;
	for (const { window } of overlays.values()) {
		if (window.isDestroyed()) continue;
		window.webContents.send("overlay:hide");
		window.setIgnoreMouseEvents(true);
		window.setFocusable(false);
		window.setSkipTaskbar(true);
	}
}
function createOverlay(display) {
	const existing = overlays.get(display.id);
	if (existing && !existing.window.isDestroyed()) return existing;
	const overlayWindow = new BrowserWindow({
		...display.bounds,
		show: false,
		frame: false,
		transparent: true,
		resizable: false,
		movable: false,
		minimizable: false,
		maximizable: false,
		fullscreenable: false,
		skipTaskbar: true,
		focusable: false,
		type: process.platform === "linux" ? "toolbar" : void 0,
		title: "",
		alwaysOnTop: true,
		hasShadow: false,
		backgroundColor: "#00000000",
		webPreferences: {
			preload: path.join(__dirname, "preload.mjs"),
			sandbox: false,
			backgroundThrottling: false
		}
	});
	const view = {
		displayId: display.id,
		window: overlayWindow,
		rendererReady: false
	};
	overlays.set(display.id, view);
	overlayWindow.setMenuBarVisibility(false);
	overlayWindow.setIgnoreMouseEvents(true);
	overlayWindow.setSkipTaskbar(true);
	overlayWindow.setFocusable(false);
	overlayWindow.on("blur", hideOverlays);
	overlayWindow.on("closed", () => {
		if (overlays.get(display.id)?.window === overlayWindow) overlays.delete(display.id);
	});
	overlayWindow.webContents.on("render-process-gone", hideOverlays);
	overlayWindow.webContents.on("did-finish-load", () => {
		view.rendererReady = true;
		overlayWindow.setIgnoreMouseEvents(true);
		overlayWindow.setFocusable(false);
		overlayWindow.setSkipTaskbar(true);
		overlayWindow.showInactive();
		overlayWindow.setFocusable(false);
		overlayWindow.setSkipTaskbar(true);
		if (overlayActive) {
			overlayWindow.webContents.send("overlay:show", audio.getState().sounds);
			overlayWindow.setFocusable(true);
			overlayWindow.setIgnoreMouseEvents(false);
		} else overlayWindow.webContents.send("overlay:hide");
	});
	loadRenderer(overlayWindow, {
		overlay: "1",
		display: String(display.id)
	});
	return view;
}
function syncOverlays() {
	const displays = screen.getAllDisplays();
	const activeIds = new Set(displays.map((display) => display.id));
	for (const [displayId, view] of overlays) {
		if (activeIds.has(displayId)) continue;
		view.window.destroy();
		overlays.delete(displayId);
	}
	for (const display of displays) if (!overlays.has(display.id)) createOverlay(display);
}
function recreateOverlay(display) {
	overlays.get(display.id)?.window.destroy();
	overlays.delete(display.id);
	createOverlay(display);
}
function showOverlay() {
	syncOverlays();
	if (overlayActive) {
		hideOverlays();
		return;
	}
	overlayActive = true;
	for (const view of overlays.values()) {
		if (view.window.isDestroyed()) continue;
		if (view.rendererReady) view.window.webContents.send("overlay:show", audio.getState().sounds);
		view.window.setSkipTaskbar(true);
		view.window.setFocusable(true);
		view.window.setIgnoreMouseEvents(false);
	}
	clearOverlayTimer();
	overlayHideTimer = setTimeout(hideOverlays, OVERLAY_TIMEOUT_MS);
}
function overlayForWebContents(webContentsId) {
	return [...overlays.values()].find(({ window }) => window.webContents.id === webContentsId);
}
function registerOverlayIpc() {
	ipcMain.on("overlay:pointer-ready", (event) => {
		const selectedOverlay = overlayForWebContents(event.sender.id);
		if (!selectedOverlay || !overlayActive) return;
		for (const view of overlays.values()) {
			if (view === selectedOverlay || view.window.isDestroyed()) continue;
			view.window.webContents.send("overlay:hide");
			view.window.setIgnoreMouseEvents(true);
			view.window.setFocusable(false);
			view.window.setSkipTaskbar(true);
		}
		selectedOverlay.window.setSkipTaskbar(true);
		selectedOverlay.window.focus();
	});
	ipcMain.on("overlay:hide", (event) => {
		if (overlayForWebContents(event.sender.id)) hideOverlays();
	});
	ipcMain.on("overlay:selected", (event, url) => {
		const selectedOverlay = overlayForWebContents(event.sender.id);
		if (!selectedOverlay) return;
		const sound = audio.getState().sounds.find((candidate) => candidate.url === url);
		if (!sound) return;
		for (const view of overlays.values()) {
			if (view === selectedOverlay || view.window.isDestroyed()) continue;
			view.window.webContents.send("overlay:stop-playback");
		}
		win?.webContents.send("main:sound-playing", sound.name);
		hideOverlays();
	});
	ipcMain.on("overlay:playback-ended", (event) => {
		if (!overlayForWebContents(event.sender.id)) return;
		win?.webContents.send("main:sound-playing", null);
	});
}
function registerShortcut() {
	if (shortcutRegistered) return;
	shortcutRegistered = globalShortcut.register(GLOBAL_SHORTCUT, () => {
		showOverlay();
	});
}
app.on("window-all-closed", () => {});
app.on("activate", () => showWindow());
var cleaningUp = false;
function cleanupAndExit(exitCode = 0) {
	if (cleaningUp) return;
	cleaningUp = true;
	globalShortcut.unregisterAll();
	hideOverlays();
	audio.cleanup().finally(() => process.exit(exitCode));
}
app.on("before-quit", (event) => {
	event.preventDefault();
	cleanupAndExit(0);
});
process.on("SIGINT", () => {
	isQuitting = true;
	cleanupAndExit(0);
});
process.on("SIGTERM", () => {
	isQuitting = true;
	cleanupAndExit(0);
});
app.whenReady().then(async () => {
	await audio.init();
	registerAudioIpc(audio);
	createWindow();
	syncOverlays();
	screen.on("display-added", (_event, display) => createOverlay(display));
	screen.on("display-removed", (_event, display) => {
		overlays.get(display.id)?.window.destroy();
		overlays.delete(display.id);
	});
	screen.on("display-metrics-changed", (_event, display) => recreateOverlay(display));
	createTray();
	registerOverlayIpc();
	registerShortcut();
});
//#endregion
export {};
