import { useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { SoundFile } from "../../audio-globals";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { emojiFontFamily } from "../Soundboard/emojiFont";
import { useI18n } from "../../i18n";

const PAGE_SIZE = 8;
const MENU_RADIUS = 230;
const PIE_CENTER = 230;
const PIE_OUTER_RADIUS = 218;
const PIE_INNER_RADIUS = 69;
const PIE_GAP_RADIANS = 0;

interface OverlayPosition {
  x: number;
  y: number;
}

function labelFor(name: string): string {
  return name.replace(/\.[^.]+$/, "");
}

function soundLabel(sound: SoundFile): string {
  return sound.displayName || labelFor(sound.name);
}

function shortLabel(sound: SoundFile): string {
  const label = soundLabel(sound);
  return label.length > 15 ? `${label.slice(0, 14)}…` : label;
}

function polarPoint(radius: number, angle: number): { x: number; y: number } {
  return {
    x: PIE_CENTER + Math.cos(angle) * radius,
    y: PIE_CENTER + Math.sin(angle) * radius,
  };
}

function pieSlicePath(startAngle: number, endAngle: number): string {
  const outerStart = polarPoint(PIE_OUTER_RADIUS, startAngle);
  const outerEnd = polarPoint(PIE_OUTER_RADIUS, endAngle);
  const innerEnd = polarPoint(PIE_INNER_RADIUS, endAngle);
  const innerStart = polarPoint(PIE_INNER_RADIUS, startAngle);
  const largeArc = endAngle - startAngle > Math.PI ? 1 : 0;

  return [
    `M ${outerStart.x} ${outerStart.y}`,
    `A ${PIE_OUTER_RADIUS} ${PIE_OUTER_RADIUS} 0 ${largeArc} 1 ${outerEnd.x} ${outerEnd.y}`,
    `L ${innerEnd.x} ${innerEnd.y}`,
    `A ${PIE_INNER_RADIUS} ${PIE_INNER_RADIUS} 0 ${largeArc} 0 ${innerStart.x} ${innerStart.y}`,
    "Z",
  ].join(" ");
}

function Overlay() {
  const { t } = useI18n();
  const [active, setActive] = useState(false);
  const [sounds, setSounds] = useState<SoundFile[]>([]);
  const [page, setPage] = useState(0);
  const [playing, setPlaying] = useState<string | null>(null);
  const [position, setPosition] = useState<OverlayPosition | null>(null);
  const positionRef = useRef<OverlayPosition | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const playbackIdRef = useRef(0);
  const hoveredSoundRef = useRef<SoundFile | null>(null);

  const pageCount = Math.max(1, Math.ceil(sounds.length / PAGE_SIZE));
  const playingSound = playing ? sounds.find((sound) => sound.url === playing) : null;
  const visibleSounds = useMemo(
    () => sounds.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE),
    [page, sounds],
  );

  useEffect(() => {
    const onShow = (_event: unknown, ...args: unknown[]) => {
      setSounds((args[0] as SoundFile[]) ?? []);
      setPage(0);
      positionRef.current = null;
      hoveredSoundRef.current = null;
      setPosition(null);
      setActive(true);
    };
    window.ipcRenderer?.on("overlay:show", onShow);
    const onHide = () => {
      setActive(false);
      positionRef.current = null;
      hoveredSoundRef.current = null;
      setPosition(null);
    };
    window.ipcRenderer?.on("overlay:hide", onHide);
    const stopPlayback = () => {
      playbackIdRef.current += 1;
      const current = audioRef.current;
      if (!current) return;
      current.onended = null;
      current.onerror = null;
      current.pause();
      current.removeAttribute("src");
      audioRef.current = null;
      setPlaying(null);
    };
    window.ipcRenderer?.on("overlay:stop-playback", stopPlayback);
    const confirmSelection = () => {
      const hoveredSound = hoveredSoundRef.current;
      if (hoveredSound) {
        play(hoveredSound);
      } else {
        window.overlay?.hide();
      }
    };
    window.ipcRenderer?.on("overlay:confirm-selection", confirmSelection);

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") window.overlay?.hide();
    };
    window.addEventListener("keydown", onKeyDown);

    return () => {
      playbackIdRef.current += 1;
      const current = audioRef.current;
      if (current) {
        current.onended = null;
        current.onerror = null;
        current.pause();
        current.removeAttribute("src");
        audioRef.current = null;
      }
      window.ipcRenderer?.off("overlay:show", onShow);
      window.ipcRenderer?.off("overlay:hide", onHide);
      window.ipcRenderer?.off("overlay:stop-playback", stopPlayback);
      window.ipcRenderer?.off("overlay:confirm-selection", confirmSelection);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  function capturePointer(event: React.PointerEvent<HTMLElement>): void {
    if (!active || positionRef.current) return;

    const x = Math.min(Math.max(event.clientX, MENU_RADIUS), window.innerWidth - MENU_RADIUS);
    const y = Math.min(Math.max(event.clientY, MENU_RADIUS), window.innerHeight - MENU_RADIUS);
    const nextPosition = { x, y };
    positionRef.current = nextPosition;
    setPosition(nextPosition);
    window.overlay?.pointerReady();
  }

  function closeOutside(event: React.PointerEvent<HTMLElement>): void {
    if (!active || event.target !== event.currentTarget) return;
    window.overlay?.hide();
  }

  function play(sound: SoundFile): void {
    const previous = audioRef.current;
    if (previous) {
      previous.onended = null;
      previous.onerror = null;
      previous.pause();
      previous.removeAttribute("src");
    }

    const playbackId = playbackIdRef.current + 1;
    playbackIdRef.current = playbackId;
    const player = new Audio(sound.url);
    audioRef.current = player;
    setPlaying(sound.url);

    const finish = () => {
      if (playbackIdRef.current !== playbackId || audioRef.current !== player) return;
      player.onended = null;
      player.onerror = null;
      audioRef.current = null;
      setPlaying(null);
      window.overlay?.playbackEnded();
    };

    player.onended = finish;
    player.onerror = finish;
    void player
      .play()
      .then(() => window.overlay?.selected(sound.url))
      .catch(finish);
  }

  function changePage(offset: number): void {
    hoveredSoundRef.current = null;
    setPage((current) => (current + offset + pageCount) % pageCount);
  }

  return (
    <main
      className={`overlay-shell${active ? " is-active" : ""}${position ? " is-positioned" : ""}`}
      aria-label={t("overlay.aria")}
      onPointerEnter={capturePointer}
      onPointerMove={capturePointer}
      onPointerDown={closeOutside}
    >
      {active && position && (
        <div
          className="overlay-anchor"
          style={{ "--left": `${position.x}px`, "--top": `${position.y}px` } as CSSProperties}
        >
          <div className="overlay-halo" />
          <section className="overlay-menu">
            <svg
              className="overlay-pie"
              viewBox="0 0 460 460"
              role="group"
              aria-label={t("overlay.soundsAria")}
            >
              {visibleSounds.map((sound, index) => {
                const sliceAngle = (Math.PI * 2) / Math.max(visibleSounds.length, 1);
                const startAngle = index * sliceAngle - Math.PI / 2 + PIE_GAP_RADIANS / 2;
                const endAngle = (index + 1) * sliceAngle - Math.PI / 2 - PIE_GAP_RADIANS / 2;
                const labelAngle = (startAngle + endAngle) / 2;
                const labelPoint = polarPoint(
                  (PIE_OUTER_RADIUS + PIE_INNER_RADIUS) / 2,
                  labelAngle,
                );
                const isPlaying = playing === sound.url;

                return (
                  <g
                    key={sound.url}
                    className={`overlay-slice${isPlaying ? " is-playing" : ""}`}
                    role="button"
                    tabIndex={0}
                    aria-label={t("overlay.playAria", { label: soundLabel(sound) })}
                    onPointerEnter={() => {
                      hoveredSoundRef.current = sound;
                    }}
                    onPointerLeave={() => {
                      if (hoveredSoundRef.current === sound) hoveredSoundRef.current = null;
                    }}
                    onFocus={() => {
                      hoveredSoundRef.current = sound;
                    }}
                    onBlur={() => {
                      if (hoveredSoundRef.current === sound) hoveredSoundRef.current = null;
                    }}
                    onClick={() => play(sound)}
                    onKeyDown={(event) => {
                      if (event.key !== "Enter" && event.key !== " ") return;
                      event.preventDefault();
                      play(sound);
                    }}
                  >
                    <path className="overlay-slice-shape" d={pieSlicePath(startAngle, endAngle)} />
                    <text
                      className="overlay-slice-label"
                      x={labelPoint.x}
                      y={labelPoint.y + 3}
                      textAnchor="middle"
                      aria-hidden="true"
                    >
                      <tspan
                        x={labelPoint.x}
                        dy="-11"
                        className="overlay-slice-icon"
                        style={{ fontFamily: emojiFontFamily }}
                      >
                        {sound.emoji}
                      </tspan>
                      <tspan x={labelPoint.x} dy="24">
                        {shortLabel(sound)}
                      </tspan>
                    </text>
                  </g>
                );
              })}
            </svg>

            <div className="overlay-center">
              <span className={`overlay-kicker${playing ? " is-live" : ""}`}>
                {playing ? t("overlay.playing") : t("overlay.soundboard")}
              </span>
              {sounds.length === 0 ? (
                <strong>{t("overlay.noSounds")}</strong>
              ) : playingSound ? (
                <strong title={soundLabel(playingSound)}>{soundLabel(playingSound)}</strong>
              ) : (
                <strong>{t("overlay.pick")}</strong>
              )}
              {pageCount > 1 && (
                <div className="overlay-pages">
                  <button
                    type="button"
                    aria-label={t("overlay.prevPage")}
                    onClick={() => changePage(-1)}
                  >
                    <ChevronLeft size={14} />
                  </button>
                  <span>
                    {page + 1}/{pageCount}
                  </span>
                  <button
                    type="button"
                    aria-label={t("overlay.nextPage")}
                    onClick={() => changePage(1)}
                  >
                    <ChevronRight size={14} />
                  </button>
                </div>
              )}
            </div>
          </section>
        </div>
      )}
    </main>
  );
}

export default Overlay;
