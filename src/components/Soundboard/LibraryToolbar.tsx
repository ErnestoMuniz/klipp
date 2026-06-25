import { LayoutGrid, Music, Plus, Rows3 } from "lucide-react";
import type { Density, Prefs, SortMode } from "./types";
import { cx } from "./styles";
import { Badge, Button, IconTile, Select } from "./ui";

interface LibraryToolbarProps {
  density: Density;
  disabled: boolean;
  prefs: Prefs;
  query: string;
  soundCount: number;
  visibleCount: number;
  onAddSounds: () => void;
  onPrefsChange: (updater: (prefs: Prefs) => Prefs) => void;
}

export function LibraryToolbar({
  density,
  disabled,
  prefs,
  query,
  soundCount,
  visibleCount,
  onAddSounds,
  onPrefsChange,
}: LibraryToolbarProps) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 px-0.5 pb-0.5 pt-1.5">
      <div className="inline-flex items-center gap-3 text-sm font-semibold text-(--text-h)">
        <IconTile size="md" aria-hidden="true">
          <Music size={13} />
        </IconTile>
        <span className="font-mono text-xs font-semibold uppercase tracking-widest text-(--text)">
          Biblioteca
        </span>
        <Badge>
          {visibleCount}
          {query && visibleCount !== soundCount ? `/${soundCount}` : ""}
        </Badge>
      </div>

      <div className="inline-flex flex-wrap items-center gap-2.5">
        <div
          className="hidden gap-1 rounded-md border border-(--border) bg-(--surface-sunk) p-1 shadow-(--inset-lo) sm:inline-flex"
          role="group"
          aria-label="Densidade da grelha"
        >
          <DensityButton
            active={density === "comfort"}
            title="Vista confortável"
            onClick={() => onPrefsChange((current) => ({ ...current, density: "comfort" }))}
          >
            <LayoutGrid size={14} />
          </DensityButton>
          <DensityButton
            active={density === "compact"}
            title="Vista compacta"
            onClick={() => onPrefsChange((current) => ({ ...current, density: "compact" }))}
          >
            <Rows3 size={14} />
          </DensityButton>
        </div>

        <Select
          className="py-2 text-sm font-medium"
          label="Ordenar"
          value={prefs.sort}
          onChange={(event) =>
            onPrefsChange((current) => ({ ...current, sort: event.target.value as SortMode }))
          }
        >
          <option value="name-asc">Nome (A-Z)</option>
          <option value="name-desc">Nome (Z-A)</option>
          <option value="recent">Recentes</option>
        </Select>

        <Button
          variant="ghost"
          className="hidden sm:block"
          onClick={() =>
            onPrefsChange((current) => ({ ...current, showHints: !current.showHints }))
          }
          aria-pressed={prefs.showHints}
          title="Mostrar/ocultar dicas"
        >
          {prefs.showHints ? "Ocultar dicas" : "Mostrar dicas"}
        </Button>

        <Button variant="primary" onClick={onAddSounds} disabled={disabled}>
          <Plus size={16} aria-hidden="true" />
          Adicionar
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
