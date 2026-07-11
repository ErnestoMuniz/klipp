import type { SoundFile } from "../../audio-globals";
import type { Density } from "./types";
import { cx } from "./styles";
import { EmptyState } from "./EmptyState";
import { SoundPad } from "./SoundPad";

interface SoundStageProps {
  density: Density;
  disabled: boolean;
  playing: string | null;
  query: string;
  sounds: SoundFile[];
  totalSounds: number;
  onAddSounds: () => void;
  onDelete: (sound: SoundFile) => void;
  onEdit: (sound: SoundFile) => void;
  onPlay: (url: string) => void;
  onToggleOverlay: (sound: SoundFile) => void;
}

export function SoundStage({
  density,
  disabled,
  playing,
  query,
  sounds,
  totalSounds,
  onAddSounds,
  onDelete,
  onEdit,
  onPlay,
  onToggleOverlay,
}: SoundStageProps) {
  if (totalSounds === 0) {
    return (
      <main className="flex-1">
        <EmptyState type="empty" disabled={disabled} onAddSounds={onAddSounds} />
      </main>
    );
  }

  if (sounds.length === 0) {
    return (
      <main className="flex-1">
        <EmptyState type="search" query={query} />
      </main>
    );
  }

  return (
    <main className="flex-1">
      <div
        className={cx(
          "grid",
          density === "comfort"
            ? "grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3.5"
            : "grid-cols-[repeat(auto-fill,minmax(118px,1fr))] gap-2.5",
        )}
      >
        {sounds.map((sound) => (
          <SoundPad
            key={sound.url}
            density={density}
            isPlaying={playing === sound.url}
            sound={sound}
            onDelete={onDelete}
            onEdit={onEdit}
            onPlay={onPlay}
            onToggleOverlay={onToggleOverlay}
          />
        ))}
      </div>
    </main>
  );
}
