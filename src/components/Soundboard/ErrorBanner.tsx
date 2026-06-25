import { AlertCircle } from "lucide-react";
import { Button, Card } from "./ui";

interface ErrorBannerProps {
  disabled: boolean;
  error: string;
  onRetry: () => void;
}

export function ErrorBanner({ disabled, error, onRetry }: ErrorBannerProps) {
  return (
    <Card
      tone="alert"
      className="flex flex-wrap items-center justify-between gap-3 px-4 py-3 text-sm text-[var(--text-h)]"
    >
      <span
        className="grid size-6 shrink-0 place-items-center text-[var(--accent)]"
        aria-hidden="true"
      >
        <AlertCircle size={13} />
      </span>
      <span className="min-w-[200px] flex-1">
        <strong>Erro:</strong> {error}
      </span>
      <Button variant="secondary" onClick={onRetry} disabled={disabled}>
        Tentar novamente
      </Button>
    </Card>
  );
}
