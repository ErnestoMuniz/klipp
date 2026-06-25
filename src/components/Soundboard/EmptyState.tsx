import { Music, Plus, Search } from "lucide-react";
import { Button, Card, IconTile } from "./ui";

interface EmptyStateProps {
  disabled?: boolean;
  query?: string;
  type: "empty" | "search";
  onAddSounds?: () => void;
}

export function EmptyState({ disabled = false, query = "", type, onAddSounds }: EmptyStateProps) {
  const isSearch = type === "search";

  return (
    <Card
      tone="dashed"
      className="flex flex-col items-center gap-3 px-5 py-16 text-center text-sm text-[var(--text)]"
    >
      <IconTile size="lg" aria-hidden="true">
        {isSearch ? <Search size={24} /> : <Music size={24} />}
      </IconTile>
      <h2 className="mt-1.5 text-lg font-semibold text-[var(--text-h)]">
        {isSearch ? "Nada encontrado" : "A sua biblioteca está vazia"}
      </h2>
      {isSearch ? (
        <p className="max-w-[420px] leading-normal">Nenhum áudio corresponde a “{query}”.</p>
      ) : (
        <>
          <p className="max-w-[420px] leading-normal">
            Adicione ficheiros de áudio com “Adicionar” ou coloque-os em{" "}
            <code className="text-xs">public/sounds/</code> e reinicie.
          </p>
          <Button className="mt-2" variant="primary" onClick={onAddSounds} disabled={disabled}>
            <Plus size={16} aria-hidden="true" />
            Adicionar áudios
          </Button>
        </>
      )}
    </Card>
  );
}
