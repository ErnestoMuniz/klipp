import type { SoundFile } from "../../audio-globals";
import type { Density } from "./types";
import { cx } from "./styles";
import { Equalizer } from "./Equalizer";
import { initials, labelFor } from "./types";

interface SoundPadProps {
  density: Density;
  isPlaying: boolean;
  sound: SoundFile;
  onPlay: (url: string, name: string) => void;
}

export function SoundPad({ density, isPlaying, sound, onPlay }: SoundPadProps) {
  return (
    <button
      type="button"
      className={cx(
        "group relative flex overflow-hidden border border-(--border) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] text-left text-(--text-h) shadow-[var(--shadow-sm),var(--inset-hi)] transition-[border-color,transform,box-shadow] duration-150 before:pointer-events-none before:absolute before:left-3 before:right-3 before:top-0 before:h-px before:bg-[linear-gradient(90deg,transparent,var(--border-strong),transparent)] before:opacity-60 after:pointer-events-none after:absolute after:inset-0 after:bg-[radial-gradient(120%_90%_at_100%_0,var(--accent-bg),transparent_55%)] after:opacity-0 after:transition-opacity hover:border-(--accent-border) hover:shadow-[var(--shadow),var(--inset-hi)] hover:after:opacity-100 hover:cursor-pointer active:translate-y-px active:scale-[0.985] active:shadow-(--inset-lo)",
        density === "comfort"
          ? "aspect-square flex-col gap-3 rounded-lg p-3.5"
          : "flex-row items-center gap-2.5 rounded-xl p-2.5",
        isPlaying &&
          "border-(--accent) bg-[linear-gradient(180deg,var(--accent-bg-strong),var(--accent-bg))] shadow-[0_0_0_3px_var(--accent-bg),0_8px_24px_-12px_var(--accent-glow),var(--inset-hi)] after:opacity-0",
      )}
      onClick={() => onPlay(sound.url, sound.name)}
      title={labelFor(sound.name)}
    >
      <span
        className={cx(
          "relative grid min-h-0 place-items-center overflow-hidden rounded-xl border border-(--border) bg-(--surface-sunk) shadow-(--inset-lo) transition group-hover:border-(--accent-border) after:absolute after:right-2 after:top-2 after:size-1 after:rounded-full after:bg-(--border-strong) after:transition",
          density === "comfort" ? "flex-1" : "size-10 shrink-0 self-start",
          isPlaying &&
            "border-(--accent-deep) bg-[linear-gradient(160deg,var(--accent),var(--accent-deep))] shadow-[inset_0_2px_6px_rgba(0,0,0,0.2)] after:bg-(--accent-ink) after:shadow-[0_0_6px_1px_var(--accent-ink)]",
        )}
        aria-hidden="true"
      >
        {isPlaying ? (
          <Equalizer />
        ) : (
          <span
            className={cx(
              "font-mono font-semibold text-(--text-faint) transition group-hover:text-(--accent)",
              density === "comfort" ? "text-xl" : "text-sm",
            )}
          >
            {initials(sound.name)}
          </span>
        )}
      </span>
      <span
        className={cx(
          "min-w-0 wrap-break-word text-sm font-medium leading-tight text-(--text-h) [display:-webkit-box] [-webkit-box-orient:vertical] overflow-hidden",
          density === "comfort" ? "line-clamp-2 shrink-0" : "flex-1 [-webkit-line-clamp:1]",
        )}
      >
        {labelFor(sound.name)}
      </span>
      <span
        className={cx(
          "shrink-0 font-mono text-xs font-semibold uppercase tracking-widest text-(--text-faint)",
          isPlaying && "text-(--accent)",
          density === "compact" && "hidden",
        )}
        aria-hidden="true"
      >
        {isPlaying ? "A tocar" : "Tocar"}
      </span>
    </button>
  );
}
