import { useEffect, useMemo, useRef, useState } from "react";
import { Plus } from "lucide-react";
import type { AudioApi, AudioState, SoundFile, SoundMetadata } from "../../audio-globals";
import { BrandLogo } from "./BrandLogo";
import { ErrorBanner } from "./ErrorBanner";
import { LibraryToolbar } from "./LibraryToolbar";
import { SettingsDrawer } from "./SettingsDrawer";
import { ShortcutBanner } from "./ShortcutBanner";
import { SoundEditor } from "./SoundEditor";
import { SoundStage } from "./SoundStage";
import { Topbar } from "./Topbar";
import { OnlineSounds } from "./OnlineSounds";
import { TransportBar } from "./TransportBar";
import { Button } from "./ui";
import { cx } from "./styles";
import { PREFS_KEY, emptyState, loadPrefs, soundLabel } from "./types";
import type { Prefs, Status, Theme } from "./types";
import { applyTheme } from "./theme";
import { useI18n } from "../../i18n";
import { useShortcut } from "./useShortcut";
import { useAppSettings } from "./useAppSettings";

function Soundboard() {
  const { t } = useI18n();
  const [state, setState] = useState<AudioState | null>(null);
  const [status, setStatus] = useState<Status>("idle");
  const [playing, setPlaying] = useState<string | null>(null);
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs);
  const [volume, setVolume] = useState(prefs.volume);
  const [muted, setMuted] = useState(prefs.muted);
  const [theme, setTheme] = useState<Theme>(prefs.theme);
  const [fatal, setFatal] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [onlineOpen, setOnlineOpen] = useState(false);
  const [draggingSounds, setDraggingSounds] = useState(false);
  const [editingSound, setEditingSound] = useState<SoundFile | null>(null);
  const [query, setQuery] = useState("");
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const playbackIdRef = useRef(0);
  const dragDepthRef = useRef(0);

  const api: AudioApi | undefined = window.audio;
  const { shortcut, set: setShortcut } = useShortcut();
  const { settings: appSettings, set: setAppSettings } = useAppSettings();

  useEffect(() => {
    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
    } catch {
      /* ignore */
    }
  }, [prefs]);

  // Apply theme whenever it changes
  useEffect(() => {
    applyTheme(theme);
    setPrefs((current) => (current.theme === theme ? current : { ...current, theme }));
  }, [theme]);

  // Listen for system theme changes when in "system" mode
  useEffect(() => {
    if (theme !== "system") return;
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => applyTheme("system");
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, [theme]);

  useEffect(() => {
    setPrefs((current) =>
      current.volume === volume && current.muted === muted
        ? current
        : { ...current, volume, muted },
    );
  }, [volume, muted]);

  useEffect(() => {
    if (!api) {
      setFatal(t("app.fatalAudio"));
      return;
    }
    void api
      .getState()
      .then(setState)
      .catch((error) =>
        setState((previous) => ({
          ...(previous ?? emptyState),
          error: error instanceof Error ? error.message : String(error),
        })),
      );
    // `t` is needed when api is missing so the fatal message matches the
    // active locale, but it otherwise only re-runs when api changes.
  }, [api, t]);

  useEffect(() => {
    if (!window.ipcRenderer) return;
    const handler = (_event: unknown, ...args: unknown[]) => {
      const [next] = args;
      if (next && typeof next === "object") setState(next as AudioState);
    };
    window.ipcRenderer.on("audio:state-changed", handler);
    return () => window.ipcRenderer?.off("audio:state-changed", handler);
  }, []);

  useEffect(() => {
    if (audioRef.current) audioRef.current.volume = muted ? 0 : volume;
  }, [volume, muted]);

  useEffect(() => {
    if (!window.ipcRenderer) return;
    const handler = (_event: unknown, ...args: unknown[]) => {
      if (audioRef.current) return;
      setPlaying((args[0] as string | null) ?? null);
    };
    window.ipcRenderer.on("main:sound-playing", handler);
    return () => window.ipcRenderer?.off("main:sound-playing", handler);
  }, []);

  useEffect(
    () => () => {
      playbackIdRef.current += 1;
      const current = audioRef.current;
      if (!current) return;
      current.onended = null;
      current.onerror = null;
      current.pause();
      current.removeAttribute("src");
      audioRef.current = null;
    },
    [],
  );

  async function run<T>(fn: () => Promise<T>): Promise<T | null> {
    setStatus("busy");
    try {
      return await fn();
    } catch (error) {
      setState((previous) => ({
        ...(previous ?? emptyState),
        error: error instanceof Error ? error.message : String(error),
      }));
      return null;
    } finally {
      setStatus("idle");
    }
  }

  async function refresh() {
    if (!api) return;
    const next = await run(() => api.getState());
    if (next) setState(next);
  }

  async function onMicSource(name: string) {
    if (!api) return;
    const next = await run(() => api.setMicSource(name));
    if (next) setState(next);
  }

  async function onMicPassthrough(enabled: boolean) {
    if (!api) return;
    const next = await run(() => api.setMicPassthrough(enabled));
    if (next) setState(next);
  }

  async function onHearClips(enabled: boolean) {
    if (!api) return;
    const next = await run(() => api.setHearClips(enabled));
    if (next) setState(next);
  }

  async function onAddSounds() {
    if (!api) return;
    const next = await run(() => api.addSounds());
    if (next) setState(next);
  }

  async function onDropSounds(event: React.DragEvent<HTMLElement>) {
    event.preventDefault();
    dragDepthRef.current = 0;
    setDraggingSounds(false);
    if (!api || disabled) return;
    const files = [...event.dataTransfer.files];
    if (files.length === 0) return;
    const next = await run(() => api.importSounds(files));
    if (next) setState(next);
  }

  async function onSaveSoundMetadata(sound: SoundFile, metadata: SoundMetadata) {
    if (!api) return;
    const next = await run(() => api.updateSoundMetadata(sound.url, metadata));
    if (next) {
      setState(next);
      setEditingSound(null);
    }
  }

  async function onDeleteSound(sound: SoundFile) {
    if (!api || !window.confirm(t("pad.deleteConfirm", { label: soundLabel(sound) }))) return;
    if (playing === sound.url) stop();
    if (editingSound?.url === sound.url) setEditingSound(null);
    const next = await run(() => api.deleteSound(sound.url));
    if (next) setState(next);
  }

  async function onToggleOverlay(sound: SoundFile) {
    if (!api) return;
    const next = await run(() =>
      api.updateSoundMetadata(sound.url, { inOverlay: !sound.inOverlay }),
    );
    if (next) setState(next);
  }

  function play(url: string) {
    const previous = audioRef.current;
    if (previous) {
      previous.onended = null;
      previous.onerror = null;
      previous.pause();
      previous.removeAttribute("src");
    }

    const playbackId = playbackIdRef.current + 1;
    playbackIdRef.current = playbackId;
    const element = new Audio(url);
    element.volume = muted ? 0 : volume;
    audioRef.current = element;
    setPlaying(url);

    const finish = () => {
      if (playbackIdRef.current !== playbackId || audioRef.current !== element) return;
      element.onended = null;
      element.onerror = null;
      audioRef.current = null;
      setPlaying(null);
    };

    element.onended = finish;
    element.onerror = finish;
    void element.play().catch(finish);
  }

  function stop() {
    playbackIdRef.current += 1;
    const current = audioRef.current;
    if (current) {
      current.onended = null;
      current.onerror = null;
      current.pause();
      current.removeAttribute("src");
      audioRef.current = null;
    }
    setPlaying(null);
  }

  const soundboardState = state ?? emptyState;
  const filtered = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    let list = normalizedQuery
      ? soundboardState.sounds.filter((sound) =>
          soundLabel(sound).toLowerCase().includes(normalizedQuery),
        )
      : soundboardState.sounds;
    if (prefs.onlyOverlay) {
      list = list.filter((sound) => sound.inOverlay);
    }
    const sorted = [...list];
    switch (prefs.sort) {
      case "name-desc":
        sorted.sort((a, b) => soundLabel(b).localeCompare(soundLabel(a)));
        break;
      case "recent":
        sorted.reverse();
        break;
      default:
        sorted.sort((a, b) => soundLabel(a).localeCompare(soundLabel(b)));
    }
    return sorted;
  }, [soundboardState.sounds, query, prefs.sort, prefs.onlyOverlay]);

  if (fatal) {
    return (
      <section
        id="soundboard"
        className="relative z-1 mx-auto flex min-h-svh w-full flex-col items-center justify-center gap-4 px-6 py-10 text-left"
      >
        <div className="flex max-w-md flex-col items-center gap-5">
          <BrandLogo large />
          <h1 className="m-0 text-6xl font-semibold text-(--text-h) max-lg:text-4xl">Klipp</h1>
          <ErrorBanner disabled={false} error={fatal} onRetry={() => window.location.reload()} />
        </div>
      </section>
    );
  }

  const disabled = status === "busy" || !soundboardState.ready;
  const volumePct = Math.round(volume * 100);
  const soundCount = soundboardState.sounds.length;
  const effectiveMuted = muted || volumePct === 0;
  const visibleCount = filtered.length;
  const playingSound = playing
    ? soundboardState.sounds.find((sound) => sound.url === playing)
    : null;

  return (
    <section
      id="soundboard"
      className={cx(
        "relative z-1 mx-auto flex min-h-svh w-full flex-col gap-4 px-6 pb-32 text-left max-sm:px-3.5 max-sm:pb-36",
      )}
      onDragEnter={(event) => {
        if (!event.dataTransfer.types.includes("Files")) return;
        event.preventDefault();
        dragDepthRef.current += 1;
        setDraggingSounds(true);
      }}
      onDragOver={(event) => {
        if (!event.dataTransfer.types.includes("Files")) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "copy";
      }}
      onDragLeave={() => {
        dragDepthRef.current = Math.max(0, dragDepthRef.current - 1);
        if (dragDepthRef.current === 0) setDraggingSounds(false);
      }}
      onDrop={(event) => void onDropSounds(event)}
    >
      {draggingSounds && (
        <div className="pointer-events-none fixed inset-4 z-50 grid place-items-center rounded-xl border-2 border-dashed border-(--accent) bg-(--bg)/90 text-lg font-semibold text-(--accent) backdrop-blur-sm">
          {t("dropSounds.label")}
        </div>
      )}
      <Topbar />

      {soundboardState.error && (
        <ErrorBanner
          disabled={disabled}
          error={soundboardState.error}
          onRetry={() => void refresh()}
        />
      )}

      <LibraryToolbar
        density={prefs.density}
        prefs={prefs}
        query={query}
        onlineOpen={onlineOpen}
        settingsOpen={settingsOpen}
        soundCount={soundCount}
        visibleCount={visibleCount}
        onPrefsChange={setPrefs}
        onQueryChange={setQuery}
        onOnlineToggle={() => setOnlineOpen((value) => !value)}
        onSettingsToggle={() => setSettingsOpen((value) => !value)}
      />

      {prefs.showHints && soundCount > 0 && (
        <ShortcutBanner
          shortcut={shortcut}
          onDismiss={() => setPrefs((current) => ({ ...current, showHints: false }))}
        />
      )}

      <SoundStage
        density={prefs.density}
        disabled={disabled}
        playing={playing}
        query={query}
        sounds={filtered}
        totalSounds={soundCount}
        onAddSounds={() => void onAddSounds()}
        onDelete={(sound) => void onDeleteSound(sound)}
        onEdit={setEditingSound}
        onPlay={play}
        onToggleOverlay={(sound) => void onToggleOverlay(sound)}
      />

      <SoundEditor
        disabled={disabled}
        sound={editingSound}
        onClose={() => setEditingSound(null)}
        onSave={(sound, metadata) => void onSaveSoundMetadata(sound, metadata)}
      />

      <OnlineSounds
        open={onlineOpen}
        onLibraryChanged={setState}
        onClose={() => setOnlineOpen(false)}
      />

      <SettingsDrawer
        disabled={disabled}
        open={settingsOpen}
        state={soundboardState}
        theme={theme}
        shortcut={shortcut}
        runInBackground={appSettings.runInBackground}
        onClose={() => setSettingsOpen(false)}
        onHearClips={(enabled) => void onHearClips(enabled)}
        onMicPassthrough={(enabled) => void onMicPassthrough(enabled)}
        onMicSource={(name) => void onMicSource(name)}
        onThemeChange={setTheme}
        onShortcutChange={setShortcut}
        onRunInBackgroundChange={(enabled) => void setAppSettings({ runInBackground: enabled })}
      />

      <TransportBar
        effectiveMuted={effectiveMuted}
        playing={playingSound ? soundLabel(playingSound) : null}
        volume={volume}
        volumePct={volumePct}
        onMutedChange={setMuted}
        onStop={stop}
        onVolumeChange={(nextVolume) => {
          setMuted(false);
          setVolume(nextVolume);
        }}
      />

      <Button
        variant="primary"
        className="fixed right-6 bottom-28 z-10 size-14 gap-0 rounded-full! p-0 shadow-[var(--shadow-lg),var(--inset-hi)] max-sm:bottom-28"
        onClick={() => void onAddSounds()}
        disabled={disabled}
        aria-label={t("app.addSoundsAria")}
        title={t("app.addSoundsTitle")}
      >
        <Plus size={26} aria-hidden="true" />
      </Button>
    </section>
  );
}

export default Soundboard;
