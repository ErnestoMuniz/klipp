import { Command, X } from "lucide-react";
import { Button, Card, IconTile } from "./ui";
import { useI18n } from "../../i18n";

interface ShortcutBannerProps {
  onDismiss: () => void;
  shortcut: string;
}

export function ShortcutBanner({ onDismiss, shortcut }: ShortcutBannerProps) {
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
        <code className="border border-(--border) bg-(--surface) font-semibold">{shortcut}</code>
        {t("shortcut.hintMiddle")}
        <code className="border border-(--border) bg-(--surface) font-semibold">Esc</code>
        {t("shortcut.hintSuffix")}
      </span>
      <Button
        variant="ghost"
        aria-label={t("shortcut.dismiss")}
        title={t("shortcut.dismiss")}
        onClick={onDismiss}
      >
        <X size={15} />
      </Button>
    </Card>
  );
}
