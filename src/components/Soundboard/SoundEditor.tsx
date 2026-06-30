import { useEffect, useMemo, useState } from "react";
import { Search, X } from "lucide-react";
import type { SoundFile, SoundMetadata } from "../../audio-globals";
import { EMOJI_GROUPS } from "./emojiData";
import type { EmojiOption } from "./emojiData";
import { EMOJI_SEARCH_INDEX } from "./emojiSearchData";
import { emojiFontFamily } from "./emojiFont";
import { Button, FieldGroup, Switch } from "./ui";
import { soundLabel } from "./types";
import { cx } from "./styles";
import { useI18n } from "../../i18n";

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
  const { t, locale } = useI18n();
  const [displayName, setDisplayName] = useState("");
  const [emoji, setEmoji] = useState("♪");
  const [inOverlay, setInOverlay] = useState(true);
  const [activeGroup, setActiveGroup] = useState<EmojiGroupLabel>(DEFAULT_EMOJI_GROUP);
  const [searchQuery, setSearchQuery] = useState("");

  const localizedIndex = EMOJI_SEARCH_INDEX[locale];

  const searchResults = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) return null;
    const results: EmojiOption[] = [];
    for (const group of EMOJI_GROUPS) {
      for (const subgroup of group.subgroups) {
        for (const option of subgroup.emojis) {
          // Prefer the search index for the app's selected language (lowercased
          // localized name + CLDR keywords); fall back to the English name.
          const haystack =
            localizedIndex?.[option.emoji]?.toLowerCase() ?? option.name.toLowerCase();
          if (haystack.includes(query)) results.push(option);
        }
      }
    }
    return results;
  }, [searchQuery, localizedIndex]);

  useEffect(() => {
    if (!sound) return;
    setDisplayName(soundLabel(sound));
    setEmoji(sound.emoji || "♪");
    setInOverlay(sound.inOverlay !== false);
  }, [sound]);

  if (!sound) return null;

  const isSearching = searchResults !== null;

  return (
    <div
      className="fixed inset-0 z-30 grid place-items-center bg-black/45 px-4"
      role="dialog"
      aria-modal="true"
      aria-label={t("editor.title")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <form
        className="flex h-[min(820px,calc(100svh-32px))] w-full max-w-4xl flex-col gap-5 overflow-hidden rounded-lg border border-(--border-strong) bg-(--surface) p-5 text-(--text-h) shadow-(--shadow) max-sm:h-[calc(100svh-20px)] max-sm:p-4"
        onSubmit={(event) => {
          event.preventDefault();
          onSave(sound, {
            displayName,
            emoji: firstGrapheme(emoji) || "♪",
            inOverlay,
          });
        }}
      >
        <header className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <h2 className="m-0 text-lg font-semibold">{t("editor.title")}</h2>
            <p className="m-0 truncate text-sm text-(--text-faint)">{sound.name}</p>
          </div>
          <Button aria-label={t("editor.closeAria")} size="sm" variant="ghost" onClick={onClose}>
            <X size={22} />
          </Button>
        </header>

        <FieldGroup label={t("editor.overlayGroup")}>
          <Switch
            checked={inOverlay}
            disabled={disabled}
            label={t("editor.overlayLabel")}
            onChange={(event) => setInOverlay(event.target.checked)}
          />
        </FieldGroup>

        <label className="flex flex-col gap-2 text-sm font-medium text-(--text-h)">
          {t("editor.name")}
          <input
            className="min-h-11 rounded-sm border border-(--border) bg-(--surface-sunk) px-3 text-base text-(--text-h) shadow-(--inset-lo) outline-none transition placeholder:text-(--text-faint) focus:border-(--accent-border)"
            autoFocus
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
            placeholder={t("editor.namePlaceholder")}
          />
        </label>

        <div className="flex min-h-0 flex-1 flex-col gap-3">
          <div className="flex items-center justify-between gap-3">
            <div className="font-mono text-xs font-semibold uppercase tracking-widest text-(--text-faint)">
              {t("editor.emoji")}
            </div>
            <div className="relative max-w-64 flex-1">
              <Search
                size={16}
                className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-(--text-faint)"
              />
              <input
                className="h-8 w-full rounded-sm border border-(--border) bg-(--surface-sunk) pr-2.5 pl-8 text-sm text-(--text-h) shadow-(--inset-lo) outline-none transition placeholder:text-(--text-faint) focus:border-(--accent-border)"
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder={t("editor.searchPlaceholder")}
                aria-label={t("editor.searchAria")}
              />
            </div>
          </div>
          {!isSearching && (
            <div className="flex shrink-0 gap-1 overflow-x-auto rounded-sm border border-(--border) bg-(--surface-sunk) p-1 shadow-(--inset-lo) scrollbar-none [&::-webkit-scrollbar]:hidden">
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
          )}
          <div
            className={cx(
              "min-h-0 flex-1 overflow-y-auto rounded-sm border border-(--border) bg-(--surface-sunk) p-2 shadow-(--inset-lo)",
              emojiPickerScrollbarClass,
            )}
          >
            {isSearching ? (
              searchResults!.length ? (
                <div className="grid grid-cols-16 gap-1.5 max-lg:grid-cols-14 max-md:grid-cols-12 max-sm:grid-cols-10">
                  {searchResults!.map((option) => (
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
              ) : (
                <p className="py-6 m-0 text-center text-sm text-(--text-faint)">
                  {t("editor.noResults")}
                </p>
              )
            ) : (
              EMOJI_GROUPS.find((group) => group.label === activeGroup)!.subgroups.map(
                (subgroup) => (
                  <section key={subgroup.label} className="mb-3 last:mb-0">
                    <h3 className="m-0 mb-1.5 font-mono text-[11px] font-semibold uppercase tracking-widest text-(--text-faint)">
                      {subgroup.label}
                    </h3>
                    <div className="grid grid-cols-16 gap-1.5 max-lg:grid-cols-14 max-md:grid-cols-12 max-sm:grid-cols-10">
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
              )
            )}
          </div>
        </div>

        <footer className="mt-1 flex shrink-0 justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            {t("editor.cancel")}
          </Button>
          <Button disabled={disabled} type="submit">
            {t("editor.save")}
          </Button>
        </footer>
      </form>
    </div>
  );
}
