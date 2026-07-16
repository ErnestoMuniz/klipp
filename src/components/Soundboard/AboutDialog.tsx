import { X } from "lucide-react";
import { useEffect, useState } from "react";
import { useI18n } from "../../i18n";
import { BrandLogo } from "./BrandLogo";
import { Button } from "./ui";

const REPOSITORY_URL = "https://github.com/ErnestoMuniz/klipp";

function GitHubLogo() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
      <path d="M12 .7a11.5 11.5 0 0 0-3.64 22.41c.58.11.79-.25.79-.56v-2.23c-3.22.7-3.9-1.37-3.9-1.37-.53-1.34-1.29-1.7-1.29-1.7-1.05-.72.08-.71.08-.71 1.17.08 1.78 1.2 1.78 1.2 1.04 1.77 2.72 1.26 3.38.96.1-.75.4-1.26.74-1.55-2.57-.29-5.27-1.28-5.27-5.68 0-1.26.45-2.28 1.19-3.09-.12-.29-.52-1.47.11-3.05 0 0 .97-.31 3.16 1.18A10.96 10.96 0 0 1 12 6.12c.98 0 1.95.13 2.86.39 2.2-1.49 3.16-1.18 3.16-1.18.63 1.58.23 2.76.11 3.05.74.81 1.19 1.83 1.19 3.09 0 4.41-2.71 5.38-5.28 5.67.42.36.78 1.06.78 2.14v3.27c0 .31.21.68.79.56A11.5 11.5 0 0 0 12 .7Z" />
    </svg>
  );
}

interface AboutDialogProps {
  open: boolean;
  onClose: () => void;
}

export function AboutDialog({ open, onClose }: AboutDialogProps) {
  const { t } = useI18n();
  const [version, setVersion] = useState("");

  useEffect(() => {
    if (open) void window.appInfo?.getVersion().then(setVersion);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-30 grid place-items-center bg-black/45 px-4 backdrop-blur-[2px]"
      role="dialog"
      aria-modal="true"
      aria-label={t("about.title")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="relative flex w-full max-w-md flex-col items-center gap-5 rounded-lg border border-(--border-strong) bg-(--surface) p-7 text-center shadow-(--shadow-lg)">
        <Button
          variant="ghost"
          className="absolute right-3 top-3"
          onClick={onClose}
          aria-label={t("about.closeAria")}
        >
          <X size={20} />
        </Button>
        <BrandLogo large />
        <div>
          <h2 className="text-2xl font-semibold text-(--text-h)">Klipp</h2>
          <p className="mt-1 text-sm text-(--text-faint)">{t("about.version", { version })}</p>
        </div>
        <p className="max-w-sm text-sm leading-relaxed text-(--text)">{t("about.description")}</p>
        <dl className="grid w-full grid-cols-[auto_1fr] gap-x-4 gap-y-2 border-y border-(--border) py-4 text-left text-sm">
          <dt className="text-(--text-faint)">{t("about.creator")}</dt>
          <dd className="font-medium text-(--text-h)">Ernesto Muniz</dd>
          <dt className="text-(--text-faint)">{t("about.platform")}</dt>
          <dd className="font-medium text-(--text-h)">Linux</dd>
          <dt className="text-(--text-faint)">{t("about.technology")}</dt>
          <dd className="font-medium text-(--text-h)">Electron · React · Vite</dd>
        </dl>
        <Button
          variant="secondary"
          className="inline-flex h-10 w-full shrink-0 items-center justify-center gap-2 py-0"
          onClick={() => void window.appInfo?.openExternal(REPOSITORY_URL)}
        >
          <GitHubLogo />
          {t("about.github")}
        </Button>
      </div>
    </div>
  );
}
