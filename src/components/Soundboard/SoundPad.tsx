import type { SoundFile } from "../../audio-globals";
import type { Density } from "./types";
import { cx } from "./styles";
import { Equalizer } from "./Equalizer";
import { Edit3, Star, StarOff } from "lucide-react";
import { emojiFontFamily } from "./emojiFont";
import { soundLabel } from "./types";
import { useI18n } from "../../i18n";

interface SoundPadProps {
  density: Density;
  isPlaying: boolean;
  sound: SoundFile;
  onEdit: (sound: SoundFile) => void;
  onPlay: (url: string) => void;
  onToggleOverlay: (sound: SoundFile) => void;
}

export function SoundPad({
  density,
  isPlaying,
  sound,
  onEdit,
  onPlay,
  onToggleOverlay,
}: SoundPadProps) {
  const { t } = useI18n();
  const label = soundLabel(sound);

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
        !sound.inOverlay && "opacity-50 grayscale",
      )}
      onClick={() => onPlay(sound.url)}
      title={label}
    >
      <span className="absolute right-2 top-2 z-1 inline-flex gap-1 opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100">
        <span
          className={cx(
            "grid size-8 cursor-pointer place-items-center rounded-sm border border-(--border) bg-(--surface-sunk) shadow-(--inset-lo) transition hover:border-(--accent-border)",
            sound.inOverlay ? "text-(--accent)" : "text-(--text-faint)",
          )}
          role="button"
          tabIndex={0}
          title={sound.inOverlay ? t("pad.removeFromPicker") : t("pad.addToPicker")}
          aria-pressed={sound.inOverlay}
          onClick={(event) => {
            event.stopPropagation();
            onToggleOverlay(sound);
          }}
          onKeyDown={(event) => {
            if (event.key !== "Enter" && event.key !== " ") return;
            event.preventDefault();
            event.stopPropagation();
            onToggleOverlay(sound);
          }}
        >
          {sound.inOverlay ? <Star size={15} /> : <StarOff size={15} />}
        </span>
        <span
          className="grid size-8 cursor-pointer place-items-center rounded-sm border border-(--border) bg-(--surface-sunk) text-(--text-faint) shadow-(--inset-lo) transition hover:border-(--accent-border) hover:text-(--accent)"
          role="button"
          tabIndex={0}
          title={t("pad.edit")}
          onClick={(event) => {
            event.stopPropagation();
            onEdit(sound);
          }}
          onKeyDown={(event) => {
            if (event.key !== "Enter" && event.key !== " ") return;
            event.preventDefault();
            event.stopPropagation();
            onEdit(sound);
          }}
        >
          <Edit3 size={15} />
        </span>
      </span>
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
              "font-semibold text-(--text-h) transition group-hover:text-(--accent)",
              density === "comfort" ? "text-4xl" : "text-xl",
            )}
            style={{ fontFamily: emojiFontFamily }}
          >
            {sound.emoji}
          </span>
        )}
      </span>
      <span
        className={cx(
          "min-w-0 wrap-break-word text-sm font-medium leading-tight text-(--text-h) [display:-webkit-box] [-webkit-box-orient:vertical] overflow-hidden",
          density === "comfort" ? "line-clamp-2 shrink-0" : "flex-1 [-webkit-line-clamp:1]",
          !sound.inOverlay && "text-(--text-faint) font-normal",
        )}
      >
        {label}
      </span>
      <span
        className={cx(
          "shrink-0 font-mono text-xs font-semibold uppercase tracking-widest text-(--text-faint)",
          isPlaying && "text-(--accent)",
          density === "compact" && "hidden",
        )}
        aria-hidden="true"
      >
        {isPlaying ? t("pad.playing") : t("pad.play")}
      </span>
    </button>
  );
}
