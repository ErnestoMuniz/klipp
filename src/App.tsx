import { useEffect, useRef, useState } from "react";
import type { AudioApi, AudioState } from "./audio-globals";
import "./App.css";

type Status = "idle" | "busy";

const GLOBAL_SHORTCUT = "Alt+Shift+S";

const emptyState: AudioState = {
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

function App() {
  const [state, setState] = useState<AudioState | null>(null);
  const [status, setStatus] = useState<Status>("idle");
  const [playing, setPlaying] = useState<string | null>(null);
  const [volume, setVolume] = useState(1);
  const [fatal, setFatal] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const playbackIdRef = useRef(0);

  const api: AudioApi | undefined = window.audio;

  useEffect(() => {
    if (!api) {
      setFatal("A API de áudio não está disponível (preload não carregou). Reinicie o app.");
      return;
    }
    void api
      .getState()
      .then(setState)
      .catch((e) =>
        setState((prev) => ({
          ...(prev ?? emptyState),
          error: e instanceof Error ? e.message : String(e),
        })),
      );
  }, [api]);

  useEffect(() => {
    if (audioRef.current) audioRef.current.volume = volume;
  }, [volume]);

  useEffect(() => {
    if (!window.ipcRenderer) return;
    const handler = (_event: unknown, ...args: unknown[]) => {
      // Uma reprodução local é a fonte da verdade enquanto estiver ativa.
      // Eventos atrasados do overlay não podem apagar o seu estado.
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
    } catch (e) {
      setState((prev) => ({
        ...(prev ?? emptyState),
        error: e instanceof Error ? e.message : String(e),
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
    const el = new Audio(url);
    el.volume = volume;
    audioRef.current = el;
    setPlaying(name);

    const finish = () => {
      if (playbackIdRef.current !== playbackId || audioRef.current !== el) return;
      el.onended = null;
      el.onerror = null;
      audioRef.current = null;
      setPlaying(null);
    };

    el.onended = finish;
    el.onerror = finish;
    void el.play().catch(finish);
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

  if (fatal) {
    return (
      <section id="soundboard">
        <header className="sb-header">
          <h1>Soundboard</h1>
        </header>
        <div className="sb-error">{fatal}</div>
      </section>
    );
  }

  const s = state ?? emptyState;
  const disabled = status === "busy" || !s.ready;

  return (
    <section id="soundboard">
      <header className="sb-header">
        <h1>Soundboard</h1>
        <p>Microfone virtual para o Discord — toca áudios e mantém o seu microfone real.</p>
      </header>

      {s.error && (
        <div className="sb-error">
          <strong>Erro:</strong> {s.error}
          <button type="button" onClick={() => void refresh()} disabled={disabled}>
            Tentar novamente
          </button>
        </div>
      )}

      <div className="sb-banner">
        No Discord, defina o <strong>dispositivo de entrada</strong> (microfone) como:
        <code>{s.discordDeviceName}</code>
      </div>

      <div className="sb-banner sb-shortcut">
        <span>
          Pressione <code>{GLOBAL_SHORTCUT}</code> em qualquer lugar para abrir o seletor rápido.
          Clique em um áudio para tocá-lo; <code>Esc</code> fecha sem selecionar.
        </span>
      </div>

      <div className="sb-card">
        <div className="sb-card-title">Microfone real (pass-through)</div>
        <label className="sb-row">
          <input
            type="checkbox"
            checked={s.micPassthrough}
            onChange={(e) => void onMicPassthrough(e.target.checked)}
            disabled={disabled || s.micSources.length === 0}
          />
          <span>Passar o meu microfone junto com os áudios</span>
        </label>
        <select
          className="sb-select"
          value={s.currentMicSource ?? ""}
          onChange={(e) => void onMicSource(e.target.value)}
          disabled={disabled || s.micSources.length === 0}
        >
          {s.micSources.length === 0 && <option value="">Nenhum microfone encontrado</option>}
          {s.micSources.map((m) => (
            <option key={m.name} value={m.name}>
              {m.description}
            </option>
          ))}
        </select>
      </div>

      <div className="sb-card">
        <div className="sb-card-title">Ouvir os áudios</div>
        <label className="sb-row">
          <input
            type="checkbox"
            checked={s.hearClips}
            onChange={(e) => void onHearClips(e.target.checked)}
            disabled={disabled}
          />
          <span>Tocar os áudios também nos meus fones/caixas (só os áudios, sem a minha voz)</span>
        </label>
      </div>

      <div className="sb-card">
        <div className="sb-card-title">
          Áudios
          <button
            type="button"
            className="sb-add"
            onClick={() => void onAddSounds()}
            disabled={disabled}
          >
            + Adicionar
          </button>
        </div>

        <div className="sb-volume">
          <span>Volume</span>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={volume}
            onChange={(e) => setVolume(Number(e.target.value))}
          />
          <button type="button" onClick={stop} disabled={!playing}>
            Parar
          </button>
        </div>

        {s.sounds.length === 0 ? (
          <p className="sb-empty">
            Nenhum áudio ainda. Solte ficheiros em <code>public/sounds/</code> ou clique em
            “Adicionar”.
          </p>
        ) : (
          <div className="sb-grid">
            {s.sounds.map((f) => (
              <button
                key={f.url}
                type="button"
                className={`sb-pad${playing === f.name ? " active" : ""}`}
                onClick={() => play(f.url, f.name)}
              >
                {playing === f.name ? "▶ " : ""}
                {f.name}
              </button>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

export default App;
