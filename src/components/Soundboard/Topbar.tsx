import { Search, Settings, X } from "lucide-react";
import { cx } from "./styles";
import { BrandLogo } from "./BrandLogo";
import { Badge, Button } from "./ui";

interface TopbarProps {
  query: string;
  ready: boolean;
  settingsOpen: boolean;
  onQueryChange: (query: string) => void;
  onSettingsToggle: () => void;
}

export function Topbar({
  query,
  ready,
  settingsOpen,
  onQueryChange,
  onSettingsToggle,
}: TopbarProps) {
  return (
    <header className="sticky top-0 z-[8] -mx-3.5 flex items-center gap-3.5 border-b border-[var(--hairline)] bg-[linear-gradient(180deg,color-mix(in_srgb,var(--bg)_92%,transparent),color-mix(in_srgb,var(--bg)_78%,transparent))] px-3.5 py-3.5 backdrop-blur-[14px] backdrop-saturate-[1.1]">
      <div className="inline-flex shrink-0 items-center gap-3">
        <BrandLogo />
        <div className="hidden flex-col gap-0.5 leading-none sm:flex">
          <span className="text-lg font-semibold text-[var(--text-h)]">Klipp</span>
          <span className="font-mono text-xs font-semibold uppercase tracking-widest text-[var(--text-faint)]">
            Soundboard
          </span>
        </div>
      </div>

      <div className="flex h-10 min-w-0 flex-1 items-center gap-2 rounded border border-[var(--border)] bg-[var(--surface-sunk)] px-3 shadow-[var(--inset-lo)] transition focus-within:border-[var(--accent-border)] focus-within:bg-[var(--surface)] focus-within:shadow-[0_0_0_4px_var(--accent-bg),var(--inset-hi)]">
        <span className="shrink-0 text-[var(--text-faint)]" aria-hidden="true">
          <Search size={17} />
        </span>
        <input
          type="search"
          className="min-w-0 flex-1 border-0 bg-transparent text-sm text-[var(--text-h)] outline-none placeholder:text-[var(--text-faint)] [appearance:none] [&::-webkit-search-cancel-button]:appearance-none"
          placeholder="Pesquisar áudios…"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          aria-label="Pesquisar áudios"
        />
        {query && (
          <Button
            variant="quietIcon"
            size="sm"
            className="size-6 rounded-full border-0 bg-[var(--border)] shadow-none hover:bg-[var(--accent)] hover:text-[var(--accent-ink)]"
            onClick={() => onQueryChange("")}
            aria-label="Limpar pesquisa"
          >
            <X size={11} />
          </Button>
        )}
      </div>

      <div className="inline-flex shrink-0 items-center gap-2.5">
        <Badge
          className="px-3 py-2 uppercase"
          tone="status"
          title={ready ? "Dispositivos prontos" : "A iniciar…"}
        >
          <span
            className={cx(
              "size-2 rounded-full",
              ready
                ? "bg-[var(--good)] shadow-[0_0_0_3px_rgba(47,143,91,0.2),0_0_8px_rgba(47,143,91,0.5)]"
                : "bg-[var(--text-faint)]",
            )}
            aria-hidden="true"
          />
          <span>{ready ? "Pronto" : "A iniciar…"}</span>
        </Badge>
        <Button
          active={settingsOpen}
          variant="icon"
          onClick={onSettingsToggle}
          aria-expanded={settingsOpen}
          aria-label="Definições"
          title="Definições"
        >
          <Settings size={16} />
        </Button>
      </div>
    </header>
  );
}
