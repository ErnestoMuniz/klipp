import { Command } from "lucide-react";
import { GLOBAL_SHORTCUT } from "./types";
import { Card, IconTile } from "./ui";

export function ShortcutBanner() {
  return (
    <Card
      tone="dashed"
      className="flex flex-wrap items-center gap-3 px-3.5 py-2.5 text-sm text-(--text-h) [&_code]:border [&_code]:border-(--border) [&_code]:bg-(--surface) [&_code]:font-semibold border-yellow-300/30 bg-yellow-300/5"
    >
      <IconTile size="sm" aria-hidden="true">
        <Command size={11} />
      </IconTile>
      <span className="min-w-55 flex-1">
        Clique num pad para o tocar. Em qualquer app, <code>{GLOBAL_SHORTCUT}</code> abre o seletor
        rápido · <code>Esc</code> fecha.
      </span>
    </Card>
  );
}
