import { Command } from "lucide-react";
import { GLOBAL_SHORTCUT } from "./types";
import { Card, IconTile } from "./ui";
import { useI18n } from "../../i18n";

export function ShortcutBanner() {
  const { t } = useI18n();
  return (
    <Card
      tone="dashed"
      className="flex flex-wrap items-center gap-3 px-3.5 py-2.5 text-sm text-(--text-h) border-yellow-300/30 bg-yellow-300/5"
    >
      <IconTile size="sm" aria-hidden="true">
        <Command size={11} />
      </IconTile>
      <span className="min-w-55 flex-1">
        {t("shortcut.hintPrefix")}
        <code className="border border-(--border) bg-(--surface) font-semibold">
          {GLOBAL_SHORTCUT}
        </code>
        {t("shortcut.hintMiddle")}
        <code className="border border-(--border) bg-(--surface) font-semibold">Esc</code>
        {t("shortcut.hintSuffix")}
      </span>
    </Card>
  );
}
