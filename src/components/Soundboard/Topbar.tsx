import { Search, Settings, X } from "lucide-react";
import { BrandLogo } from "./BrandLogo";
import { Button } from "./ui";

interface TopbarProps {
  query: string;
  ready: boolean;
  settingsOpen: boolean;
  onQueryChange: (query: string) => void;
  onSettingsToggle: () => void;
}

export function Topbar({ query, settingsOpen, onQueryChange, onSettingsToggle }: TopbarProps) {
  return (
    <header className="sticky top-0 z-8 -mx-3.5 flex items-center gap-3.5 border-b border-(--hairline) bg-[linear-gradient(180deg,color-mix(in_srgb,var(--bg)_92%,transparent),color-mix(in_srgb,var(--bg)_78%,transparent))] px-3.5 py-3.5 backdrop-blur-[14px] backdrop-saturate-[1.1]">
      <div className="inline-flex shrink-0 items-center gap-3">
        <BrandLogo />
        <div className="hidden flex-col gap-0.5 leading-none sm:flex">
          <span className="text-lg font-semibold text-(--text-h)">Klipp</span>
        </div>
      </div>

      <div className="flex h-10 min-w-0 flex-1 items-center gap-2 rounded-lg border border-(--border) bg-(--surface-sunk) px-3 shadow-(--inset-lo) transition focus-within:border-(--accent-border) focus-within:bg-(--surface) focus-within:shadow-[0_0_0_4px_var(--accent-bg),var(--inset-hi)]">
        <span className="shrink-0 text-(--text-faint)" aria-hidden="true">
          <Search size={17} />
        </span>
        <input
          type="search"
          className="min-w-0 flex-1 border-0 bg-transparent text-sm text-(--text-h) outline-none placeholder:text-(--text-faint) appearance-none [&::-webkit-search-cancel-button]:appearance-none"
          placeholder="Pesquisar áudios…"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          aria-label="Pesquisar áudios"
        />
        {query && (
          <Button
            variant="quietIcon"
            size="sm"
            className="size-6 rounded-full border-0 bg-(--border) shadow-none hover:bg-(--accent) hover:text-(--accent-ink)"
            onClick={() => onQueryChange("")}
            aria-label="Limpar pesquisa"
          >
            <X size={11} />
          </Button>
        )}
      </div>

      <div className="inline-flex shrink-0 items-center gap-2.5">
        <Button
          active={settingsOpen}
          variant="ghost"
          onClick={onSettingsToggle}
          aria-expanded={settingsOpen}
          aria-label="Definições"
          title="Definições"
        >
          <Settings size={22} />
        </Button>
      </div>
    </header>
  );
}
