import { Moon, Sun, SunMoon, X } from "lucide-react";
import type { AudioState } from "../../audio-globals";
import { GLOBAL_SHORTCUT } from "./types";
import type { Theme } from "./types";
import { cx, dividerClass } from "./styles";
import { Button, FieldGroup, Select, Switch } from "./ui";
import { themeLabels } from "./theme";

interface SettingsDrawerProps {
  disabled: boolean;
  open: boolean;
  state: AudioState;
  theme: Theme;
  onClose: () => void;
  onHearClips: (enabled: boolean) => void;
  onMicPassthrough: (enabled: boolean) => void;
  onMicSource: (name: string) => void;
  onThemeChange: (theme: Theme) => void;
}

export function SettingsDrawer({
  disabled,
  open,
  state,
  theme,
  onClose,
  onHearClips,
  onMicPassthrough,
  onMicSource,
  onThemeChange,
}: SettingsDrawerProps) {
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
        aria-label="Definições"
      >
        <div className="flex items-center justify-between border-b border-(--border) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] px-5 pb-2 pt-2 shadow-(--inset-hi)">
          <span className="text-lg font-semibold text-(--text-h)">Definições</span>
          <Button variant="ghost" onClick={onClose} aria-label="Fechar definições">
            <X size={22} />
          </Button>
        </div>

        <div className="flex flex-col gap-5 overflow-y-auto p-5">
          <FieldGroup label="Microfone real (pass-through)">
            <Switch
              checked={state.micPassthrough}
              disabled={disabled || state.micSources.length === 0}
              label="Passar o meu microfone junto com os áudios"
              onChange={(event) => onMicPassthrough(event.target.checked)}
            />
            <Select
              label="Microfone"
              value={state.currentMicSource ?? ""}
              onChange={(event) => onMicSource(event.target.value)}
              disabled={disabled || state.micSources.length === 0}
            >
              {state.micSources.length === 0 && (
                <option value="">Nenhum microfone encontrado</option>
              )}
              {state.micSources.map((mic) => (
                <option key={mic.name} value={mic.name}>
                  {mic.description}
                </option>
              ))}
            </Select>
          </FieldGroup>

          <div className={dividerClass} />

          <FieldGroup label="Monitorização">
            <Switch
              checked={state.hearClips}
              disabled={disabled}
              label="Ouvir os áudios nos meus fones/caixas (só os áudios, sem a minha voz)"
              onChange={(event) => onHearClips(event.target.checked)}
            />
          </FieldGroup>

          <div className={dividerClass} />

          <FieldGroup label="Tema">
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

          <div className="flex flex-col gap-2">
            <FieldGroup label="Dispositivo no Discord">
              <p className="text-sm leading-normal text-(--text)">
                Defina o <strong className="text-(--text-h)">dispositivo de entrada</strong> como{" "}
                <code className="text-xs">{state.discordDeviceName}</code>.
              </p>
              <p className="text-sm leading-normal text-(--text)">
                Atalho global <code className="text-xs">{GLOBAL_SHORTCUT}</code> abre o seletor
                rápido · <code className="text-xs">Esc</code> fecha.
              </p>
            </FieldGroup>
          </div>
        </div>
      </aside>
    </>
  );
}
