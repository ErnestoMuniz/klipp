import { Music, Plus, Search } from "lucide-react";
import { Button, Card, IconTile } from "./ui";
import { useI18n } from "../../i18n";

interface EmptyStateProps {
  disabled?: boolean;
  query?: string;
  type: "empty" | "search";
  onAddSounds?: () => void;
}

export function EmptyState({ disabled = false, query = "", type, onAddSounds }: EmptyStateProps) {
  const { t } = useI18n();
  const isSearch = type === "search";

  return (
    <Card
      tone="dashed"
      className="flex flex-col items-center gap-3 px-5 py-16 text-center text-sm text-(--text)"
    >
      <IconTile size="lg" aria-hidden="true">
        {isSearch ? <Search size={24} /> : <Music size={24} />}
      </IconTile>
      <h2 className="mt-1.5 text-lg font-semibold text-(--text-h)">
        {isSearch ? t("empty.searchTitle") : t("empty.libraryTitle")}
      </h2>
      {isSearch ? (
        <p className="max-w-105 leading-normal">{t("empty.searchBody", { query })}</p>
      ) : (
        <>
          <p className="max-w-105 leading-normal">{t("empty.libraryBody")}</p>
          <Button className="mt-2" variant="primary" onClick={onAddSounds} disabled={disabled}>
            <Plus size={16} aria-hidden="true" />
            {t("empty.addSounds")}
          </Button>
        </>
      )}
    </Card>
  );
}
