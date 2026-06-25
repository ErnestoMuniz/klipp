import { useEffect, useMemo, useRef, useState } from "react";
import type { AudioApi, AudioState } from "../../audio-globals";
import { BrandLogo } from "./BrandLogo";
import { ErrorBanner } from "./ErrorBanner";
import { LibraryToolbar } from "./LibraryToolbar";
import { SettingsDrawer } from "./SettingsDrawer";
import { ShortcutBanner } from "./ShortcutBanner";
import { SoundStage } from "./SoundStage";
import { Topbar } from "./Topbar";
import { TransportBar } from "./TransportBar";
import { cx } from "./styles";
import { PREFS_KEY, emptyState, labelFor, loadPrefs } from "./types";
import type { Prefs, Status } from "./types";

function Soundboard() {
  const [state, setState] = useState<AudioState | null>(null);
  const [status, setStatus] = useState<Status>("idle");
  const [playing, setPlaying] = useState<string | null>(null);
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs);
  const [volume, setVolume] = useState(prefs.volume);
  const [muted, setMuted] = useState(prefs.muted);
  const [fatal, setFatal] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [query, setQuery] = useState("");
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const playbackIdRef = useRef(0);

  const api: AudioApi | undefined = window.audio;

  useEffect(() => {
    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
    } catch {
      /* ignore */
    }
  }, [prefs]);

  useEffect(() => {
    setPrefs((current) =>
      current.volume === volume && current.muted === muted
        ? current
        : { ...current, volume, muted },
    );
  }, [volume, muted]);

  useEffect(() => {
    if (!api) {
      setFatal("A API de áudio não está disponível (preload não carregou). Reinicie o app.");
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
  }, [api]);

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

  function play(url: string, name: string) {
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
    setPlaying(name);

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
    const list = normalizedQuery
      ? soundboardState.sounds.filter((sound) =>
          labelFor(sound.name).toLowerCase().includes(normalizedQuery),
        )
      : soundboardState.sounds;
    const sorted = [...list];
    switch (prefs.sort) {
      case "name-desc":
        sorted.sort((a, b) => labelFor(b.name).localeCompare(labelFor(a.name)));
        break;
      case "recent":
        sorted.reverse();
        break;
      default:
        sorted.sort((a, b) => labelFor(a.name).localeCompare(labelFor(b.name)));
    }
    return sorted;
  }, [soundboardState.sounds, query, prefs.sort]);

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

  return (
    <section
      id="soundboard"
      className={cx(
        "relative z-1 mx-auto flex w-full flex-col gap-4 px-6 pb-32 text-left max-sm:px-3.5 max-sm:pb-36",
      )}
    >
      <Topbar
        query={query}
        ready={soundboardState.ready}
        settingsOpen={settingsOpen}
        onQueryChange={setQuery}
        onSettingsToggle={() => setSettingsOpen((value) => !value)}
      />

      {soundboardState.error && (
        <ErrorBanner
          disabled={disabled}
          error={soundboardState.error}
          onRetry={() => void refresh()}
        />
      )}

      <LibraryToolbar
        density={prefs.density}
        disabled={disabled}
        prefs={prefs}
        query={query}
        soundCount={soundCount}
        visibleCount={visibleCount}
        onAddSounds={() => void onAddSounds()}
        onPrefsChange={setPrefs}
      />

      {prefs.showHints && soundCount > 0 && <ShortcutBanner />}

      <SoundStage
        density={prefs.density}
        disabled={disabled}
        playing={playing}
        query={query}
        sounds={filtered}
        totalSounds={soundCount}
        onAddSounds={() => void onAddSounds()}
        onPlay={play}
      />

      <SettingsDrawer
        disabled={disabled}
        open={settingsOpen}
        state={soundboardState}
        onClose={() => setSettingsOpen(false)}
        onHearClips={(enabled) => void onHearClips(enabled)}
        onMicPassthrough={(enabled) => void onMicPassthrough(enabled)}
        onMicSource={(name) => void onMicSource(name)}
      />

      <TransportBar
        effectiveMuted={effectiveMuted}
        playing={playing}
        volume={volume}
        volumePct={volumePct}
        onMutedChange={setMuted}
        onStop={stop}
        onVolumeChange={(nextVolume) => {
          setMuted(false);
          setVolume(nextVolume);
        }}
      />
    </section>
  );
}

export default Soundboard;
