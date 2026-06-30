import { Globe, Moon, Sun, SunMoon, X } from "lucide-react";
import type { AudioState } from "../../audio-globals";
import { DEFAULT_SHORTCUT } from "./types";
import type { Theme } from "./types";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { cx, dividerClass } from "./styles";
import { Button, FieldGroup, Select, Switch } from "./ui";
import { useI18n } from "../../i18n";
import type { LanguagePref } from "../../i18n";

interface SettingsDrawerProps {
  disabled: boolean;
  open: boolean;
  state: AudioState;
  theme: Theme;
  shortcut: string;
  runInBackground: boolean;
  onClose: () => void;
  onHearClips: (enabled: boolean) => void;
  onMicPassthrough: (enabled: boolean) => void;
  onMicSource: (name: string) => void;
  onThemeChange: (theme: Theme) => void;
  onShortcutChange: (accelerator: string) => Promise<{ registered: boolean; error?: string }>;
  onRunInBackgroundChange: (enabled: boolean) => void;
}

export function SettingsDrawer({
  disabled,
  open,
  state,
  theme,
  shortcut,
  runInBackground,
  onClose,
  onHearClips,
  onMicPassthrough,
  onMicSource,
  onThemeChange,
  onShortcutChange,
  onRunInBackgroundChange,
}: SettingsDrawerProps) {
  const { t, language, setLanguage } = useI18n();

  const themeLabels: Record<Theme, string> = {
    light: t("settings.themeLight"),
    dark: t("settings.themeDark"),
    system: t("settings.themeSystem"),
  };

  const languageOptions: { value: LanguagePref; label: string }[] = [
    { value: "system", label: t("settings.languageSystem") },
    { value: "en", label: t("settings.languageEn") },
    { value: "pt-BR", label: t("settings.languagePtBR") },
  ];

  return (
    <>
      <div
        className={cx(
          "fixed inset-0 z-18 bg-[rgba(20,16,10,0.42)] opacity-0 backdrop-blur-[2px] transition-opacity duration-200",
          open ? "pointer-events-auto opacity-100" : "pointer-events-none",
        )}
        onClick={onClose}
        aria-hidden="true"
      />
      <aside
        className={cx(
          "fixed bottom-0 right-0 top-0 z-20 flex w-[min(384px,92vw)] flex-col border-l border-(--border) bg-(--surface) shadow-(--shadow-lg) transition-transform duration-300",
          open ? "translate-x-0" : "translate-x-full",
        )}
        aria-hidden={!open}
        aria-label={t("settings.title")}
      >
        <div className="flex items-center justify-between border-b border-(--border) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] px-5 pb-2 pt-2 shadow-(--inset-hi)">
          <span className="text-lg font-semibold text-(--text-h)">{t("settings.title")}</span>
          <Button variant="ghost" onClick={onClose} aria-label={t("settings.closeAria")}>
            <X size={22} />
          </Button>
        </div>

        <div className="flex flex-col gap-5 overflow-y-auto p-5">
          <FieldGroup label={t("settings.micPassthroughGroup")}>
            <Switch
              checked={state.micPassthrough}
              disabled={disabled || state.micSources.length === 0}
              label={t("settings.micPassthroughLabel")}
              onChange={(event) => onMicPassthrough(event.target.checked)}
            />
            <Select
              label={t("settings.micLabel")}
              value={state.currentMicSource ?? ""}
              onChange={(event) => onMicSource(event.target.value)}
              disabled={disabled || state.micSources.length === 0}
            >
              {state.micSources.length === 0 && <option value="">{t("settings.micNone")}</option>}
              {state.micSources.map((mic) => (
                <option key={mic.name} value={mic.name}>
                  {mic.description}
                </option>
              ))}
            </Select>
          </FieldGroup>

          <div className={dividerClass} />

          <FieldGroup label={t("settings.monitoringGroup")}>
            <Switch
              checked={state.hearClips}
              disabled={disabled}
              label={t("settings.hearClipsLabel")}
              onChange={(event) => onHearClips(event.target.checked)}
            />
          </FieldGroup>

          <div className={dividerClass} />

          <FieldGroup label={t("settings.themeGroup")}>
            <div className="flex gap-2">
              {(["light", "dark", "system"] as const).map((value) => (
                <button
                  key={value}
                  type="button"
                  className={cx(
                    "flex flex-1 flex-col items-center gap-1.5 rounded-md border px-3 py-2.5 text-xs font-medium transition-colors",
                    theme === value
                      ? "border-(--accent) bg-(--accent-bg) text-(--accent)"
                      : "border-(--border) bg-(--surface-sunk) text-(--text) hover:border-(--accent-border)",
                  )}
                  onClick={() => onThemeChange(value)}
                  disabled={disabled}
                  aria-pressed={theme === value}
                >
                  {value === "light" && <Sun size={18} />}
                  {value === "dark" && <Moon size={18} />}
                  {value === "system" && <SunMoon size={18} />}
                  <span>{themeLabels[value]}</span>
                </button>
              ))}
            </div>
          </FieldGroup>

          <FieldGroup label={t("settings.languageGroup")}>
            <div className="inline-flex items-center gap-2">
              <Globe size={16} className="text-(--text-faint)" aria-hidden="true" />
              <Select
                label={t("settings.languageGroup")}
                value={language}
                onChange={(event) => setLanguage(event.target.value as LanguagePref)}
              >
                {languageOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </Select>
            </div>
          </FieldGroup>

          <div className={dividerClass} />

          <FieldGroup label={t("settings.backgroundGroup")}>
            <Switch
              checked={runInBackground}
              disabled={disabled}
              label={t("settings.backgroundLabel")}
              onChange={(event) => onRunInBackgroundChange(event.target.checked)}
            />
            <p className="text-sm leading-normal text-(--text)">{t("settings.backgroundHint")}</p>
          </FieldGroup>

          <div className={dividerClass} />

          <FieldGroup label={t("settings.shortcutGroup")}>
            <ShortcutRecorder
              disabled={disabled}
              shortcut={shortcut}
              defaultValue={DEFAULT_SHORTCUT}
              onChange={onShortcutChange}
            />
            <p className="text-sm leading-normal text-(--text)">
              {t("settings.globalShortcutHint", {
                shortcut,
                esc: "Esc",
              })}
            </p>
          </FieldGroup>

          <div className={dividerClass} />

          <div className="flex flex-col gap-2">
            <FieldGroup label={t("settings.discordGroup")}>
              <p className="text-sm leading-normal text-(--text)">
                {t("settings.discordIntro")}{" "}
                <strong className="text-(--text-h)">{t("settings.discordInputDevice")}</strong>{" "}
                {t("settings.discordTo")} <code className="text-xs">{state.discordDeviceName}</code>
                .
              </p>
            </FieldGroup>
          </div>
        </div>
      </aside>
    </>
  );
}
