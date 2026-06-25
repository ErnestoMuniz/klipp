import { Play, Square, Volume1, Volume2, VolumeX } from "lucide-react";
import { Equalizer } from "./Equalizer";
import { labelFor } from "./types";
import { Button } from "./ui";

interface TransportBarProps {
  effectiveMuted: boolean;
  playing: string | null;
  volume: number;
  volumePct: number;
  onMutedChange: (updater: (muted: boolean) => boolean) => void;
  onStop: () => void;
  onVolumeChange: (volume: number) => void;
}

export function TransportBar({
  effectiveMuted,
  playing,
  volume,
  volumePct,
  onMutedChange,
  onStop,
  onVolumeChange,
}: TransportBarProps) {
  return (
    <footer className="fixed bottom-6 left-1/2 z-10 flex w-[min(1040px,calc(100%-48px))] -translate-x-1/2 items-center gap-3.5 rounded-lg border border-(--border) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] px-3.5 py-3 shadow-[var(--shadow-lg),var(--inset-hi)] backdrop-blur-[14px] backdrop-saturate-[1.1] max-sm:flex-wrap max-sm:gap-3">
      <Button
        variant="transport"
        size="lg"
        onClick={playing ? onStop : undefined}
        disabled={!playing}
        title={playing ? "Parar" : "Sem reprodução"}
        aria-label={playing ? "Parar reprodução" : "Sem reprodução"}
      >
        {playing ? <Square size={13} /> : <Play size={13} />}
      </Button>

      <div className="flex min-w-0 flex-1 items-center gap-3 border-r border-(--border) pr-3.5 max-sm:basis-full max-sm:border-b max-sm:border-r-0 max-sm:pb-3 max-sm:pr-0">
        <Equalizer live={Boolean(playing)} transport />
        <div className="flex min-w-0 flex-col gap-px">
          <span className="font-mono text-xs font-semibold uppercase tracking-widest text-(--text-faint)">
            {playing ? "A tocar" : "Parado"}
          </span>
          <span
            className="max-w-[38vw] overflow-hidden text-ellipsis whitespace-nowrap text-sm font-medium text-(--text-h)"
            title={playing ?? undefined}
          >
            {playing ? labelFor(playing) : "Pronto a reproduzir"}
          </span>
        </div>
      </div>

      <div className="flex min-w-44 flex-[1.4] items-center gap-3 max-sm:flex-1">
        <Button
          variant="quietIcon"
          size="sm"
          onClick={() => onMutedChange((muted) => !muted)}
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
        </Button>
        <input
          type="range"
          className="h-1.5 flex-1 accent-(--accent)"
          min={0}
          max={1}
          step={0.01}
          value={volume}
          onChange={(event) => onVolumeChange(Number(event.target.value))}
          aria-label="Volume"
        />
        <span className="min-w-10 shrink-0 text-right font-mono text-xs font-semibold tabular-nums tracking-wide text-(--text-h)">
          {volumePct}%
        </span>
      </div>
    </footer>
  );
}
