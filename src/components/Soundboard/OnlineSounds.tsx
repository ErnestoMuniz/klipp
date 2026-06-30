import { useEffect, useRef, useState } from "react";
import { Check, ChevronDown, Cloud, Download, Loader2, Pause, Play, Search, X } from "lucide-react";
import type { AudioState, RemoteSound, SearchPage } from "../../audio-globals";
import { Button } from "./ui";
import { cx } from "./styles";
import { useI18n } from "../../i18n";

interface OnlineSoundsProps {
  /** Drawer visibility. */
  open: boolean;
  /** Receives the refreshed library state after a successful download. */
  onLibraryChanged: (state: AudioState) => void;
  /** Close the drawer (backdrop click / close button). */
  onClose: () => void;
}

/**
 * Browse / preview / download sounds from myinstants.com.
 *
 * Searching happens through `window.soundsBrowser` (handled in the main
 * process — it scrapes the myinstants search pages over HTTPS so the
 * renderer neither fights CORS nor needs HTML parsing). Previewing plays the
 * remote `.mp3` URLs directly with an `<audio>` element so the rest of the
 * library's playback is untouched. Downloading streams the file into the
 * userData sounds dir via the main process and hands the refreshed library
 * state back to the parent.
 */
export function OnlineSounds({ open, onLibraryChanged, onClose }: OnlineSoundsProps) {
  const { t } = useI18n();
  const api = window.soundsBrowser;

  // `query` is the committed search string; `queryInput` is the live field.
  // We trigger a fresh search only when the user submits (Enter), not on
  // every keystroke — searching on a half-typed term churns the network.
  const [queryInput, setQueryInput] = useState("");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<RemoteSound[]>([]);
  const [page, setPage] = useState(1);
  const [hasMore, setHasMore] = useState(false);
  const [searching, setSearching] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState<string | null>(null);
  const [downloadingIds, setDownloadingIds] = useState<Set<string>>(new Set());
  const [downloadedIds, setDownloadedIds] = useState<Set<string>>(new Set());

  const audioRef = useRef<HTMLAudioElement | null>(null);
  // Monotonic request id so a stale search response (from a previous query)
  // can't overwrite the latest results when it eventually resolves.
  const searchReqRef = useRef(0);

  // Reset the field/results whenever the drawer is reopened from closed.
  useEffect(() => {
    if (open) return;
    const element = audioRef.current;
    if (element) {
      element.pause();
      audioRef.current = null;
    }
    setPreviewing(null);
  }, [open]);

  useEffect(
    () => () => {
      audioRef.current?.pause();
      audioRef.current = null;
    },
    [],
  );

  // Run a fresh search whenever the committed query changes (or the drawer
  // opens with one already set).
  useEffect(() => {
    if (!open) return;
    const trimmed = query.trim();
    if (!trimmed) {
      setResults([]);
      setHasMore(false);
      setPage(1);
      setSearching(false);
      setError(null);
      return;
    }

    const request = (searchReqRef.current += 1);
    setSearching(true);
    setError(null);
    void api
      ?.search(trimmed, 1)
      .then((value: SearchPage) => {
        if (request !== searchReqRef.current) return;
        setResults(value.results);
        setHasMore(value.hasMore);
        setPage(1);
      })
      .catch((cause: unknown) => {
        if (request !== searchReqRef.current) return;
        setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (request !== searchReqRef.current) return;
        setSearching(false);
      });
  }, [open, query, api]);

  function togglePreview(sound: RemoteSound): void {
    const element = audioRef.current;
    if (element && previewing === sound.url) {
      element.pause();
      audioRef.current = null;
      setPreviewing(null);
      return;
    }
    element?.pause();
    audioRef.current = null;

    const next = new Audio(sound.url);
    next.onended = () => {
      if (audioRef.current === next) audioRef.current = null;
      setPreviewing(null);
    };
    next.onerror = () => {
      if (audioRef.current === next) audioRef.current = null;
      setPreviewing(null);
    };
    audioRef.current = next;
    setPreviewing(sound.url);
    void next.play().catch(() => setPreviewing(null));
  }

  async function loadMore(): Promise<void> {
    if (loadingMore || !hasMore) return;
    setLoadingMore(true);
    setError(null);
    try {
      const nextPage = page + 1;
      const value = await api?.search(query.trim(), nextPage);
      if (value) {
        setResults((previous) => dedupe([...previous, ...value.results]));
        setHasMore(value.hasMore);
        setPage(nextPage);
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoadingMore(false);
    }
  }

  async function download(sound: RemoteSound): Promise<void> {
    if (downloadingIds.has(sound.id)) return;
    setDownloadingIds((current) => new Set(current).add(sound.id));
    setError(null);
    try {
      const state = await api?.download({
        url: sound.url,
        file: sound.file,
        title: sound.title,
      });
      if (state) onLibraryChanged(state);
      setDownloadedIds((current) => new Set(current).add(sound.id));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setDownloadingIds((current) => {
        const next = new Set(current);
        next.delete(sound.id);
        return next;
      });
    }
  }

  function submitQuery(event: React.FormEvent): void {
    event.preventDefault();
    const trimmed = queryInput.trim();
    setQuery(trimmed);
  }

  const trimmedQuery = query.trim();

  return (
    <>
      <div
        className={cx(
          "fixed inset-0 z-18 bg-[rgba(20,16,10,0.42)] opacity-0 backdrop-blur-[2px] transition-opacity duration-200",
          open ? "pointer-events-auto opacity-100" : "pointer-events-none",
        )}
        onClick={onClose}
        aria-hidden="true"
      />
      <aside
        className={cx(
          "fixed bottom-0 right-0 top-0 z-20 flex w-[min(420px,94vw)] flex-col border-l border-(--border) bg-(--surface) shadow-(--shadow-lg) transition-transform duration-300",
          open ? "translate-x-0" : "translate-x-full",
        )}
        aria-hidden={!open}
        aria-label={t("online.title")}
      >
        <div className="flex items-center justify-between border-b border-(--border) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] px-5 pb-2 pt-2 shadow-(--inset-hi)">
          <span className="inline-flex items-center gap-2 text-lg font-semibold text-(--text-h)">
            <Cloud size={20} aria-hidden="true" />
            {t("online.title")}
          </span>
          <Button variant="ghost" onClick={onClose} aria-label={t("online.closeAria")}>
            <X size={22} />
          </Button>
        </div>

        <form
          onSubmit={submitQuery}
          className="flex flex-col gap-2.5 border-b border-(--border) p-4"
        >
          <div className="flex h-11 items-center gap-2 rounded-md border border-(--border) bg-(--surface-sunk) px-3 shadow-(--inset-lo) transition focus-within:border-(--accent-border) focus-within:bg-(--surface) focus-within:shadow-[0_0_0_4px_var(--accent-bg),var(--inset-hi)]">
            <span className="shrink-0 text-(--text-faint)" aria-hidden="true">
              <Search size={17} />
            </span>
            <input
              type="search"
              className="min-w-0 flex-1 border-0 bg-transparent text-sm text-(--text-h) outline-none placeholder:text-(--text-faint) appearance-none [&::-webkit-search-cancel-button]:appearance-none"
              placeholder={t("online.searchPlaceholder")}
              value={queryInput}
              onChange={(event) => setQueryInput(event.target.value)}
              aria-label={t("online.searchAria")}
            />
            {queryInput && (
              <Button
                variant="quietIcon"
                size="sm"
                className="size-6 rounded-full border-0 bg-(--border) shadow-none hover:bg-(--accent) hover:text-(--accent-ink)"
                onClick={() => {
                  setQueryInput("");
                  setQuery("");
                }}
                aria-label={t("toolbar.clearSearch")}
              >
                <X size={11} />
              </Button>
            )}
          </div>
          <p className="text-xs leading-normal text-(--text-faint)">{t("online.attribution")}</p>
        </form>

        <div className="flex-1 overflow-y-auto p-3">
          {error && (
            <p className="m-2 rounded-md border border-(--accent-border) bg-(--accent-bg) px-3 py-2 text-sm text-(--accent)">
              {t("error.prefix")} {error}
            </p>
          )}

          {!error && !trimmedQuery && (
            <p className="px-2 py-16 text-center text-sm leading-relaxed text-(--text-faint)">
              {t("online.hint")}
            </p>
          )}

          {results.map((sound) => {
            const isPreviewing = previewing === sound.url;
            const isDownloading = downloadingIds.has(sound.id);
            const isDownloaded = downloadedIds.has(sound.id);
            return (
              <div
                key={sound.url}
                className={cx(
                  "mb-2 flex items-center gap-2.5 rounded-md border border-(--border) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] px-3 py-2 shadow-(--inset-hi) transition-colors hover:border-(--accent-border)",
                  isDownloaded && "border-(--accent-border)",
                )}
              >
                <Button
                  variant="quietIcon"
                  size="sm"
                  className="size-9 shrink-0"
                  onClick={() => togglePreview(sound)}
                  aria-pressed={isPreviewing}
                  aria-label={isPreviewing ? t("online.pause") : t("online.preview")}
                  title={isPreviewing ? t("online.pause") : t("online.preview")}
                >
                  {isPreviewing ? <Pause size={16} /> : <Play size={16} />}
                </Button>

                <span
                  className="min-w-0 flex-1 truncate text-sm font-medium text-(--text-h)"
                  title={sound.title}
                >
                  {sound.title || sound.file}
                </span>

                <Button
                  variant="secondary"
                  className="size-9 shrink-0 p-0"
                  onClick={() => void download(sound)}
                  disabled={isDownloading || isDownloaded}
                  aria-label={isDownloaded ? t("online.downloaded") : t("online.download")}
                  title={isDownloaded ? t("online.downloaded") : t("online.download")}
                >
                  {isDownloading ? (
                    <Loader2 size={16} className="animate-spin" />
                  ) : isDownloaded ? (
                    <Check size={16} className="text-(--accent)" />
                  ) : (
                    <Download size={16} />
                  )}
                </Button>
              </div>
            );
          })}

          {searching && results.length === 0 && (
            <p className="flex items-center justify-center gap-2 px-2 py-10 text-sm text-(--text-faint)">
              <Loader2 size={16} className="animate-spin" />
              {t("online.searching")}
            </p>
          )}

          {!searching && trimmedQuery && results.length > 0 && !hasMore && (
            <p className="px-2 py-6 text-center text-xs text-(--text-faint)">
              {t("online.endOfResults")}
            </p>
          )}

          {hasMore && results.length > 0 && (
            <Button
              variant="ghost"
              className="mt-2 w-full justify-center gap-2 py-2.5"
              onClick={() => void loadMore()}
              disabled={loadingMore}
            >
              {loadingMore ? (
                <Loader2 size={16} className="animate-spin" />
              ) : (
                <ChevronDown size={16} />
              )}
              {t("online.loadMore")}
            </Button>
          )}
        </div>
      </aside>
    </>
  );
}

/** Drop duplicate-by-URL entries so paginated concatenation stays clean. */
function dedupe(sounds: RemoteSound[]): RemoteSound[] {
  const seen = new Set<string>();
  const out: RemoteSound[] = [];
  for (const sound of sounds) {
    if (seen.has(sound.url)) continue;
    seen.add(sound.url);
    out.push(sound);
  }
  return out;
}
