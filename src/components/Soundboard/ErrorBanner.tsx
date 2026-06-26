import { AlertCircle } from "lucide-react";
import { Button, Card } from "./ui";
import { useI18n } from "../../i18n";

interface ErrorBannerProps {
  disabled: boolean;
  error: string;
  onRetry: () => void;
}

export function ErrorBanner({ disabled, error, onRetry }: ErrorBannerProps) {
  const { t } = useI18n();
  return (
    <Card
      tone="alert"
      className="flex flex-wrap items-center justify-between gap-3 px-4 py-3 text-sm text-(--text-h)"
    >
      <span className="grid size-6 shrink-0 place-items-center text-(--accent)" aria-hidden="true">
        <AlertCircle size={13} />
      </span>
      <span className="min-w-50 flex-1">
        <strong>{t("error.prefix")}</strong> {error}
      </span>
      <Button variant="secondary" onClick={onRetry} disabled={disabled}>
        {t("error.retry")}
      </Button>
    </Card>
  );
}
