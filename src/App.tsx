import { useEffect, useMemo, useRef, useState } from "react";
import type { AudioApi, AudioState } from "./audio-globals";
import {
  AlertCircle,
  Command,
  LayoutGrid,
  Music,
  Play,
  Plus,
  Rows3,
  Search,
  Settings,
  Square,
  Volume1,
  Volume2,
  VolumeX,
  X,
} from "lucide-react";
import "./App.css";

type Status = "idle" | "busy";
type Density = "comfort" | "compact";
type SortMode = "name-asc" | "name-desc" | "recent";

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

const PREFS_KEY = "klipp.prefs.v1";

interface Prefs {
  density: Density;
  sort: SortMode;
  showHints: boolean;
}

const defaultPrefs: Prefs = {
  density: "comfort",
  sort: "name-asc",
  showHints: true,
};

function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (!raw) return defaultPrefs;
    const parsed = JSON.parse(raw) as Partial<Prefs>;
    return { ...defaultPrefs, ...parsed };
  } catch {
    return defaultPrefs;
  }
}

function labelFor(name: string): string {
  return name.replace(/\.[^.]+$/, "");
}

function initials(name: string): string {
  const label = labelFor(name);
  const parts = label.split(/[\s_-]+/).filter(Boolean);
  if (parts.length === 0) return "♪";
  if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase();
  return (parts[0]![0]! + parts[1]![0]!).toUpperCase();
}

function App() {
  const [state, setState] = useState<AudioState | null>(null);
  const [status, setStatus] = useState<Status>("idle");
  const [playing, setPlaying] = useState<string | null>(null);
  const [volume, setVolume] = useState(1);
  const [fatal, setFatal] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [muted, setMuted] = useState(false);
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs);
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
    el.volume = muted ? 0 : volume;
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

  const s = state ?? emptyState;

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = q ? s.sounds.filter((f) => labelFor(f.name).toLowerCase().includes(q)) : s.sounds;
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
  }, [s.sounds, query, prefs.sort]);

  if (fatal) {
    return (
      <section id="soundboard" className="is-fatal">
        <div className="sb-empty-hero">
          <div className="sb-logo" aria-hidden="true">
            K
          </div>
          <h1>Klipp</h1>
          <div className="sb-error">{fatal}</div>
        </div>
      </section>
    );
  }

  const disabled = status === "busy" || !s.ready;
  const volumePct = Math.round(volume * 100);
  const soundCount = s.sounds.length;
  const effectiveMuted = muted || volumePct === 0;
  const density = prefs.density;
  const visibleCount = filtered.length;

  return (
    <section id="soundboard" data-density={density}>
      {/* ===== Topbar ===== */}
      <header className="sb-topbar">
        <div className="sb-brand">
          <div className="sb-logo" aria-hidden="true">
            K
          </div>
          <div className="sb-brand-text">
            <span className="sb-brand-name">Klipp</span>
            <span className="sb-brand-sub">Soundboard</span>
          </div>
        </div>

        <div className="sb-search-wrap">
          <span className="sb-search-icon" aria-hidden="true">
            <Search size={17} />
          </span>
          <input
            type="search"
            className="sb-search"
            placeholder="Pesquisar áudios…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            aria-label="Pesquisar áudios"
          />
          {query && (
            <button
              type="button"
              className="sb-search-clear"
              onClick={() => setQuery("")}
              aria-label="Limpar pesquisa"
            >
              <X size={11} />
            </button>
          )}
        </div>

        <div className="sb-topbar-actions">
          <div className="sb-status" title={s.ready ? "Dispositivos prontos" : "A iniciar…"}>
            <span className={`sb-dot${s.ready ? " is-on" : ""}`} aria-hidden="true" />
            <span>{s.ready ? "Pronto" : "A iniciar…"}</span>
          </div>
          <button
            type="button"
            className={`sb-iconbtn${settingsOpen ? " is-active" : ""}`}
            onClick={() => setSettingsOpen((v) => !v)}
            aria-expanded={settingsOpen}
            aria-label="Definições"
            title="Definições"
          >
            <span aria-hidden="true">
              <Settings size={16} />
            </span>
          </button>
        </div>
      </header>

      {s.error && (
        <div className="sb-error">
          <span className="sb-error-icon" aria-hidden="true">
            <AlertCircle size={13} />
          </span>
          <span className="sb-error-text">
            <strong>Erro:</strong> {s.error}
          </span>
          <button type="button" onClick={() => void refresh()} disabled={disabled}>
            Tentar novamente
          </button>
        </div>
      )}

      {/* ===== Toolbar da biblioteca ===== */}
      <div className="sb-toolbar">
        <div className="sb-toolbar-title">
          <span className="sb-card-icon" aria-hidden="true">
            <Music size={13} />
          </span>
          <span className="sb-toolbar-label">Biblioteca</span>
          <span className="sb-count">
            {visibleCount}
            {query && visibleCount !== soundCount ? `/${soundCount}` : ""}
          </span>
        </div>

        <div className="sb-toolbar-tools">
          <div className="sb-seg" role="group" aria-label="Densidade da grelha">
            <button
              type="button"
              className={density === "comfort" ? "is-on" : ""}
              onClick={() => setPrefs((p) => ({ ...p, density: "comfort" }))}
              aria-pressed={density === "comfort"}
              title="Vista confortável"
            >
              <LayoutGrid size={14} />
            </button>
            <button
              type="button"
              className={density === "compact" ? "is-on" : ""}
              onClick={() => setPrefs((p) => ({ ...p, density: "compact" }))}
              aria-pressed={density === "compact"}
              title="Vista compacta"
            >
              <Rows3 size={14} />
            </button>
          </div>

          <label className="sb-sort">
            <span className="sb-sr-only">Ordenar</span>
            <select
              value={prefs.sort}
              onChange={(e) => setPrefs((p) => ({ ...p, sort: e.target.value as SortMode }))}
            >
              <option value="name-asc">Nome (A–Z)</option>
              <option value="name-desc">Nome (Z–A)</option>
              <option value="recent">Recentes</option>
            </select>
          </label>

          <button
            type="button"
            className="sb-ghost"
            onClick={() => setPrefs((p) => ({ ...p, showHints: !p.showHints }))}
            aria-pressed={prefs.showHints}
            title="Mostrar/ocultar dicas"
          >
            {prefs.showHints ? "Ocultar dicas" : "Mostrar dicas"}
          </button>

          <button
            type="button"
            className="sb-add"
            onClick={() => void onAddSounds()}
            disabled={disabled}
          >
            <span aria-hidden="true">
              <Plus size={16} />
            </span>{" "}
            Adicionar
          </button>
        </div>
      </div>

      {prefs.showHints && soundCount > 0 && (
        <div className="sb-banner sb-shortcut">
          <span className="sb-banner-icon" aria-hidden="true">
            <Command size={11} />
          </span>
          <span className="sb-banner-text">
            Clique num pad para o tocar. Em qualquer app, <code>{GLOBAL_SHORTCUT}</code> abre o
            seletor rápido · <code>Esc</code> fecha.
          </span>
        </div>
      )}

      {/* ===== Palco (pads) ===== */}
      <main className="sb-stage">
        {s.sounds.length === 0 ? (
          <div className="sb-empty">
            <span className="sb-empty-icon" aria-hidden="true">
              <Music size={24} />
            </span>
            <h2>A sua biblioteca está vazia</h2>
            <p>
              Adicione ficheiros de áudio com “Adicionar” ou coloque-os em{" "}
              <code>public/sounds/</code> e reinicie.
            </p>
            <button
              type="button"
              className="sb-add"
              onClick={() => void onAddSounds()}
              disabled={disabled}
            >
              <span aria-hidden="true">
                <Plus size={16} />
              </span>{" "}
              Adicionar áudios
            </button>
          </div>
        ) : visibleCount === 0 ? (
          <div className="sb-empty">
            <span className="sb-empty-icon" aria-hidden="true">
              <Search size={24} />
            </span>
            <h2>Nada encontrado</h2>
            <p>Nenhum áudio corresponde a “{query}”.</p>
          </div>
        ) : (
          <div className="sb-grid">
            {filtered.map((f) => {
              const isPlaying = playing === f.name;
              return (
                <button
                  key={f.url}
                  type="button"
                  className={`sb-pad${isPlaying ? " is-playing" : ""}`}
                  onClick={() => play(f.url, f.name)}
                  title={labelFor(f.name)}
                >
                  <span className="sb-pad-art" aria-hidden="true">
                    {isPlaying ? (
                      <span className="sb-eq" aria-hidden="true">
                        <span />
                        <span />
                        <span />
                        <span />
                      </span>
                    ) : (
                      <span className="sb-pad-initials">{initials(f.name)}</span>
                    )}
                  </span>
                  <span className="sb-pad-name">{labelFor(f.name)}</span>
                  <span className="sb-pad-action" aria-hidden="true">
                    {isPlaying ? "A tocar" : "Tocar"}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </main>

      {/* ===== Drawer de definições ===== */}
      <div
        className={`sb-drawer-scrim${settingsOpen ? " is-open" : ""}`}
        onClick={() => setSettingsOpen(false)}
        aria-hidden="true"
      />
      <aside
        className={`sb-drawer${settingsOpen ? " is-open" : ""}`}
        aria-hidden={!settingsOpen}
        aria-label="Definições"
      >
        <div className="sb-drawer-head">
          <h2>Definições</h2>
          <button
            type="button"
            className="sb-iconbtn"
            onClick={() => setSettingsOpen(false)}
            aria-label="Fechar definições"
          >
            <X size={16} />
          </button>
        </div>

        <div className="sb-drawer-body">
          <div className="sb-group">
            <div className="sb-group-label">Microfone real (pass-through)</div>
            <label className="sb-row">
              <span className="sb-switch">
                <input
                  type="checkbox"
                  checked={s.micPassthrough}
                  onChange={(e) => void onMicPassthrough(e.target.checked)}
                  disabled={disabled || s.micSources.length === 0}
                />
                <span className="sb-switch-track" aria-hidden="true" />
              </span>
              <span className="sb-row-text">Passar o meu microfone junto com os áudios</span>
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

          <div className="sb-divider" />

          <div className="sb-group">
            <div className="sb-group-label">Monitorização</div>
            <label className="sb-row">
              <span className="sb-switch">
                <input
                  type="checkbox"
                  checked={s.hearClips}
                  onChange={(e) => void onHearClips(e.target.checked)}
                  disabled={disabled}
                />
                <span className="sb-switch-track" aria-hidden="true" />
              </span>
              <span className="sb-row-text">
                Ouvir os áudios nos meus fones/caixas (só os áudios, sem a minha voz)
              </span>
            </label>
          </div>

          <div className="sb-divider" />

          <div className="sb-group sb-hints">
            <div className="sb-group-label">Dispositivo no Discord</div>
            <p className="sb-hint-line">
              Defina o <strong>dispositivo de entrada</strong> como{" "}
              <code>{s.discordDeviceName}</code>.
            </p>
            <p className="sb-hint-line">
              Atalho global <code>{GLOBAL_SHORTCUT}</code> abre o seletor rápido · <code>Esc</code>{" "}
              fecha.
            </p>
          </div>
        </div>
      </aside>

      {/* ===== Transport bar (rodapé fixo) ===== */}
      <footer className="sb-bar">
        <button
          type="button"
          className={`sb-bar-playbtn${playing ? " is-live" : ""}`}
          onClick={playing ? stop : undefined}
          disabled={!playing}
          title={playing ? "Parar" : "Sem reprodução"}
          aria-label={playing ? "Parar reprodução" : "Sem reprodução"}
        >
          {playing ? (
            <span aria-hidden="true">
              <Square size={13} />
            </span>
          ) : (
            <span aria-hidden="true">
              <Play size={13} />
            </span>
          )}
        </button>

        <div className="sb-bar-now">
          <span className={`sb-bar-eq${playing ? " is-live" : ""}`} aria-hidden="true">
            <span />
            <span />
            <span />
            <span />
          </span>
          <div className="sb-bar-now-text">
            <span className="sb-bar-now-label">{playing ? "A tocar" : "Parado"}</span>
            <span className="sb-bar-now-name" title={playing ?? undefined}>
              {playing ? labelFor(playing) : "Pronto a reproduzir"}
            </span>
          </div>
        </div>

        <div className="sb-bar-volume">
          <button
            type="button"
            className="sb-bar-mute"
            onClick={() => setMuted((m) => !m)}
            aria-label={effectiveMuted ? "Reativar som" : "Silenciar"}
            title={effectiveMuted ? "Reativar som" : "Silenciar"}
          >
            {effectiveMuted ? (
              <VolumeX size={15} />
            ) : volumePct < 50 ? (
              <Volume1 size={15} />
            ) : (
              <Volume2 size={15} />
            )}
          </button>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={volume}
            onChange={(e) => {
              setMuted(false);
              setVolume(Number(e.target.value));
            }}
            aria-label="Volume"
          />
          <span className="sb-bar-volume-value">{volumePct}%</span>
        </div>
      </footer>
    </section>
  );
}

export default App;
