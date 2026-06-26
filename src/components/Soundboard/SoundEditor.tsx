import { useEffect, useState } from "react";
import { X } from "lucide-react";
import type { SoundFile, SoundMetadata } from "../../audio-globals";
import { EMOJI_GROUPS } from "./emojiData";
import { emojiFontFamily } from "./emojiFont";
import { Button } from "./ui";
import { soundLabel } from "./types";
import { cx } from "./styles";

const DEFAULT_EMOJI_GROUP = EMOJI_GROUPS[0]!.label;
type EmojiGroupLabel = (typeof EMOJI_GROUPS)[number]["label"];

const emojiStyle = { fontFamily: emojiFontFamily };
const emojiPickerScrollbarClass = "sb-scroll";

function firstGrapheme(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";
  if (typeof Intl.Segmenter === "function") {
    const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });
    const first = segmenter.segment(trimmed)[Symbol.iterator]().next().value as
      | Intl.SegmentData
      | undefined;
    return first?.segment ?? "";
  }
  return Array.from(trimmed)[0] ?? "";
}

interface SoundEditorProps {
  disabled: boolean;
  sound: SoundFile | null;
  onClose: () => void;
  onSave: (sound: SoundFile, metadata: SoundMetadata) => void;
}

export function SoundEditor({ disabled, sound, onClose, onSave }: SoundEditorProps) {
  const [displayName, setDisplayName] = useState("");
  const [emoji, setEmoji] = useState("♪");
  const [activeGroup, setActiveGroup] = useState<EmojiGroupLabel>(DEFAULT_EMOJI_GROUP);

  useEffect(() => {
    if (!sound) return;
    setDisplayName(soundLabel(sound));
    setEmoji(sound.emoji || "♪");
  }, [sound]);

  if (!sound) return null;

  return (
    <div
      className="fixed inset-0 z-30 grid place-items-center bg-black/45 px-4"
      role="dialog"
      aria-modal="true"
      aria-label="Editar áudio"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <form
        className="flex h-[min(820px,calc(100svh-32px))] w-full max-w-4xl flex-col gap-5 overflow-hidden rounded-lg border border-(--border-strong) bg-(--surface) p-5 text-(--text-h) shadow-(--shadow) max-sm:h-[calc(100svh-20px)] max-sm:p-4"
        onSubmit={(event) => {
          event.preventDefault();
          onSave(sound, { displayName, emoji: firstGrapheme(emoji) || "♪" });
        }}
      >
        <header className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <h2 className="m-0 text-lg font-semibold">Editar áudio</h2>
            <p className="m-0 truncate text-sm text-(--text-faint)">{sound.name}</p>
          </div>
          <Button aria-label="Fechar" size="sm" variant="ghost" onClick={onClose}>
            <X size={22} />
          </Button>
        </header>

        <label className="flex flex-col gap-2 text-sm font-medium text-(--text-h)">
          Nome
          <input
            className="min-h-11 rounded-sm border border-(--border) bg-(--surface-sunk) px-3 text-base text-(--text-h) shadow-(--inset-lo) outline-none transition placeholder:text-(--text-faint) focus:border-(--accent-border)"
            autoFocus
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
            placeholder="Nome do áudio"
          />
        </label>

        <div className="flex min-h-0 flex-1 flex-col gap-3">
          <div className="font-mono text-xs font-semibold uppercase tracking-widest text-(--text-faint)">
            Emoji
          </div>
          <div className="flex gap-1 overflow-x-auto rounded-sm border border-(--border) bg-(--surface-sunk) p-1 shadow-(--inset-lo) scrollbar-none [&::-webkit-scrollbar]:hidden">
            {EMOJI_GROUPS.map((group) => (
              <button
                key={group.label}
                type="button"
                className={cx(
                  "min-h-8 flex-1 shrink-0 cursor-pointer whitespace-nowrap rounded-sm px-2.5 py-1.5 text-xs font-semibold text-(--text-faint) transition hover:text-(--accent) hover:bg-(--surface-raise)",
                  activeGroup === group.label && "bg-(--surface-raise) text-(--text-h)",
                )}
                aria-pressed={activeGroup === group.label}
                onClick={() => setActiveGroup(group.label)}
              >
                {group.label}
              </button>
            ))}
          </div>
          <div
            className={cx(
              "min-h-70 flex-1 overflow-y-auto rounded-sm border border-(--border) bg-(--surface-sunk) p-2 shadow-(--inset-lo)",
              emojiPickerScrollbarClass,
            )}
          >
            {EMOJI_GROUPS.find((group) => group.label === activeGroup)!.subgroups.map(
              (subgroup) => (
                <section key={subgroup.label} className="mb-3 last:mb-0">
                  <h3 className="m-0 mb-1.5 font-mono text-[11px] font-semibold uppercase tracking-widest text-(--text-faint)">
                    {subgroup.label}
                  </h3>
                  <div className="grid grid-cols-12 gap-1.5 max-lg:grid-cols-10 max-md:grid-cols-8 max-sm:grid-cols-6">
                    {subgroup.emojis.map((option) => (
                      <button
                        key={option.emoji}
                        type="button"
                        className={cx(
                          "grid aspect-square min-h-11 cursor-pointer place-items-center rounded-sm border border-transparent text-2xl transition hover:border-(--accent-border) hover:bg-(--surface-raise)",
                          emoji === option.emoji &&
                            "border-(--accent) bg-(--accent-bg) shadow-[0_0_0_3px_var(--accent-bg)]",
                        )}
                        style={emojiStyle}
                        aria-label={option.name}
                        aria-pressed={emoji === option.emoji}
                        title={option.name}
                        onClick={() => setEmoji(option.emoji)}
                      >
                        {option.emoji}
                      </button>
                    ))}
                  </div>
                </section>
              ),
            )}
          </div>
        </div>

        <footer className="mt-1 flex shrink-0 justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            Cancelar
          </Button>
          <Button disabled={disabled} type="submit">
            Salvar
          </Button>
        </footer>
      </form>
    </div>
  );
}
