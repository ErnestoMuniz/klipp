import { LayoutGrid, Lightbulb, Music, Rows3, Search, Settings, Star, X } from "lucide-react";
import type { Density, Prefs, SortMode } from "./types";
import { cx } from "./styles";
import { Badge, Button, IconTile, Select } from "./ui";
import { useI18n } from "../../i18n";

interface LibraryToolbarProps {
  density: Density;
  prefs: Prefs;
  query: string;
  settingsOpen: boolean;
  soundCount: number;
  visibleCount: number;
  onPrefsChange: (updater: (prefs: Prefs) => Prefs) => void;
  onQueryChange: (query: string) => void;
  onSettingsToggle: () => void;
}

export function LibraryToolbar({
  density,
  prefs,
  query,
  settingsOpen,
  soundCount,
  visibleCount,
  onPrefsChange,
  onQueryChange,
  onSettingsToggle,
}: LibraryToolbarProps) {
  const { t } = useI18n();

  return (
    <div className="flex flex-wrap items-center justify-between gap-3 px-0.5 pb-0.5 pt-1.5">
      <div className="inline-flex items-center gap-3 text-sm font-semibold text-(--text-h)">
        <IconTile size="md" aria-hidden="true">
          <Music size={13} />
        </IconTile>
        <span className="font-mono text-xs font-semibold uppercase tracking-widest text-(--text)">
          {t("toolbar.library")}
        </span>
        <Badge>
          {visibleCount}
          {query && visibleCount !== soundCount ? `/${soundCount}` : ""}
        </Badge>
      </div>

      <div className="inline-flex flex-wrap items-center gap-2.5">
        <div className="flex h-10 min-w-56 flex-1 items-center gap-2 rounded-md border border-(--border) bg-(--surface-sunk) px-3 shadow-(--inset-lo) transition focus-within:border-(--accent-border) focus-within:bg-(--surface) focus-within:shadow-[0_0_0_4px_var(--accent-bg),var(--inset-hi)]">
          <span className="shrink-0 text-(--text-faint)" aria-hidden="true">
            <Search size={17} />
          </span>
          <input
            type="search"
            className="min-w-0 flex-1 border-0 bg-transparent text-sm text-(--text-h) outline-none placeholder:text-(--text-faint) appearance-none [&::-webkit-search-cancel-button]:appearance-none"
            placeholder={t("toolbar.searchPlaceholder")}
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            aria-label={t("toolbar.searchAria")}
          />
          {query && (
            <Button
              variant="quietIcon"
              size="sm"
              className="size-6 rounded-full border-0 bg-(--border) shadow-none hover:bg-(--accent) hover:text-(--accent-ink)"
              onClick={() => onQueryChange("")}
              aria-label={t("toolbar.clearSearch")}
            >
              <X size={11} />
            </Button>
          )}
        </div>

        <div
          className="hidden gap-1 rounded-md border border-(--border) bg-(--surface-sunk) p-1 shadow-(--inset-lo) sm:inline-flex"
          role="group"
          aria-label={t("toolbar.densityAria")}
        >
          <DensityButton
            active={density === "comfort"}
            title={t("toolbar.densityComfort")}
            onClick={() => onPrefsChange((current) => ({ ...current, density: "comfort" }))}
          >
            <LayoutGrid size={14} />
          </DensityButton>
          <DensityButton
            active={density === "compact"}
            title={t("toolbar.densityCompact")}
            onClick={() => onPrefsChange((current) => ({ ...current, density: "compact" }))}
          >
            <Rows3 size={14} />
          </DensityButton>
        </div>

        <Select
          className="py-2 text-sm font-medium"
          label={t("toolbar.sortLabel")}
          value={prefs.sort}
          onChange={(event) =>
            onPrefsChange((current) => ({ ...current, sort: event.target.value as SortMode }))
          }
        >
          <option value="name-asc">{t("toolbar.sortNameAsc")}</option>
          <option value="name-desc">{t("toolbar.sortNameDesc")}</option>
          <option value="recent">{t("toolbar.sortRecent")}</option>
        </Select>

        <Button
          variant="ghost"
          className={cx("size-10 p-0", prefs.onlyOverlay && "text-(--accent)")}
          onClick={() =>
            onPrefsChange((current) => ({ ...current, onlyOverlay: !current.onlyOverlay }))
          }
          aria-pressed={prefs.onlyOverlay}
          aria-label={t("toolbar.overlayOnlyAria")}
          title={t("toolbar.overlayOnlyTitle")}
        >
          <Star size={18} aria-hidden="true" />
        </Button>

        <Button
          variant="ghost"
          className={cx("size-10 p-0", prefs.showHints && "text-(--accent)")}
          onClick={() =>
            onPrefsChange((current) => ({ ...current, showHints: !current.showHints }))
          }
          aria-pressed={prefs.showHints}
          aria-label={prefs.showHints ? t("toolbar.hideHints") : t("toolbar.showHints")}
          title={prefs.showHints ? t("toolbar.hideHints") : t("toolbar.showHints")}
        >
          <Lightbulb size={18} aria-hidden="true" />
        </Button>

        <Button
          active={settingsOpen}
          variant="ghost"
          className="size-10 p-0"
          onClick={onSettingsToggle}
          aria-expanded={settingsOpen}
          aria-label={t("toolbar.settingsAria")}
          title={t("toolbar.settingsAria")}
        >
          <Settings size={20} />
        </Button>
      </div>
    </div>
  );
}

interface DensityButtonProps {
  active: boolean;
  children: React.ReactNode;
  onClick: () => void;
  title: string;
}

function DensityButton({ active, children, onClick, title }: DensityButtonProps) {
  return (
    <Button
      variant="ghost"
      className={cx(
        "rounded-sm p-0 text-(--text-faint)",
        active &&
          "bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] text-(--accent) shadow-[var(--shadow-sm),var(--inset-hi)]",
      )}
      onClick={onClick}
      aria-pressed={active}
      title={title}
    >
      {children}
    </Button>
  );
}
