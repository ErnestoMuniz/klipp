import { useCallback, useEffect, useRef, useState } from "react";
import { RotateCcw } from "lucide-react";
import { cx, formSelectClass } from "./styles";
import { Button } from "./ui";
import { useI18n } from "../../i18n";

interface ShortcutRecorderProps {
  disabled: boolean;
  shortcut: string;
  /** Default accelerator offered by the "reset" button. */
  defaultValue: string;
  onChange: (accelerator: string) => Promise<{ registered: boolean; error?: string }>;
}

const MODIFIER_KEYS = new Set(["Alt", "Shift", "Control", "Meta"]);

function isFunctionKey(key: string): boolean {
  return /^F([1-9]|1\d|2[0-4])$/.test(key);
}

/** Convert a DOM `KeyboardEvent.key` into an Electron accelerator key token. */
function toAcceleratorKey(key: string): string | null {
  if (key.length === 1 && /^[a-zA-Z0-9]$/u.test(key)) return key.toUpperCase();
  if (isFunctionKey(key)) return key;
  return null;
}

/**
 * Captures the next non-modifier keypress and turns it (plus whatever
 * modifiers are held) into an Electron accelerator string. Lets the user
 * rebind the global quick-picker shortcut from Settings.
 */
export function ShortcutRecorder({
  disabled,
  shortcut,
  defaultValue,
  onChange,
}: ShortcutRecorderProps) {
  const { t } = useI18n();
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const commit = useCallback(
    async (accelerator: string) => {
      const result = await onChange(accelerator);
      if (result.registered) {
        setError(null);
        return;
      }
      setError(
        result.error === "taken" ? t("settings.shortcutTaken") : t("settings.shortcutInvalid"),
      );
    },
    [onChange, t],
  );

  useEffect(() => {
    if (!recording) return;
    const controller = new AbortController();

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setRecording(false);
        return;
      }
      // Wait for a real key; pure modifier presses just prime the combo.
      if (MODIFIER_KEYS.has(event.key)) return;

      const key = toAcceleratorKey(event.key);
      if (!key) {
        setError(t("settings.shortcutInvalid"));
        setRecording(false);
        return;
      }

      const parts: string[] = [];
      if (event.altKey) parts.push("Alt");
      if (event.shiftKey) parts.push("Shift");
      if (event.ctrlKey) parts.push("Ctrl");
      if (event.metaKey) parts.push("Super");
      // Require a modifier for plain letter/digit keys so we don't hijack a
      // single common key globally; bare function keys are allowed.
      if (parts.length === 0 && !isFunctionKey(key)) {
        setError(t("settings.shortcutInvalid"));
        setRecording(false);
        return;
      }
      parts.push(key);

      setRecording(false);
      void commit(parts.join("+"));
    };

    window.addEventListener("keydown", onKeyDown, { signal: controller.signal, capture: true });
    return () => controller.abort();
  }, [recording, commit, t]);

  // Restore focus to the trigger when recording ends (e.g. after Escape).
  useEffect(() => {
    if (recording) return;
    buttonRef.current?.focus();
  }, [recording]);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <button
          ref={buttonRef}
          type="button"
          disabled={disabled}
          className={cx(
            formSelectClass,
            "flex-1 cursor-pointer font-mono",
            recording && "border-(--accent) text-(--accent)",
          )}
          aria-label={t("settings.shortcutRecordAria")}
          onClick={() => {
            setError(null);
            setRecording(true);
          }}
        >
          {recording ? t("settings.shortcutRecording") : shortcut}
        </button>
        {shortcut !== defaultValue && (
          <Button
            variant="quietIcon"
            size="md"
            disabled={disabled}
            aria-label={t("settings.shortcutReset")}
            title={t("settings.shortcutReset")}
            onClick={() => void commit(defaultValue)}
          >
            <RotateCcw size={16} aria-hidden="true" />
          </Button>
        )}
      </div>
      {error && <p className="text-xs text-(--danger, #c0392b)">{error}</p>}
    </div>
  );
}
