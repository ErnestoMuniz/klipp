import { useEffect, useState } from "react";
import { Minus, Square, Copy, X } from "lucide-react";

/**
 * Window controls (close / minimize / maximize) rendered inside the
 * frameless main window's Topbar.
 *
 * Styled as a compact "console module": a single inset tray holding three
 * square buttons separated by hairline dividers, matching the app's studio
 * aesthetic. Glyphs stay hidden until hover; minimize/maximize light up in
 * the accent blue, close signals danger in red. The whole group opts out of
 * the Topbar's drag region so each button stays clickable.
 */
export function TitleBarButtons() {
  const controls = window.windowControls;
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!window.ipcRenderer) return;
    const handler = (_event: unknown, ...args: unknown[]) => setMaximized(Boolean(args[0]));
    window.ipcRenderer.on("window:maximized", handler);
    return () => window.ipcRenderer?.off("window:maximized", handler);
  }, []);

  const available = Boolean(controls);

  return (
    <div
      className="app-no-drag mr-1 flex shrink-0 items-center gap-2"
      aria-label="Controles da janela"
    >
      <ControlButton
        label="Minimizar"
        tone="accent"
        disabled={!available}
        onClick={() => controls?.minimize()}
      >
        <Minus className="size-4" strokeWidth={2.25} />
      </ControlButton>

      <ControlButton
        label={maximized ? "Restaurar" : "Maximizar"}
        tone="accent"
        disabled={!available}
        onClick={() => controls?.toggleMaximize()}
      >
        {maximized ? (
          <Copy className="size-3" strokeWidth={2.25} />
        ) : (
          <Square className="size-3" strokeWidth={2.25} />
        )}
      </ControlButton>

      <ControlButton
        label="Fechar"
        tone="danger"
        disabled={!available}
        onClick={() => controls?.close()}
      >
        <X className="size-4" strokeWidth={2.25} />
      </ControlButton>
    </div>
  );
}

type Tone = "accent" | "danger";

interface ControlButtonProps {
  label: string;
  tone: Tone;
  disabled?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}

function ControlButton({ label, tone, disabled, onClick, children }: ControlButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className={cx(
        "tb-btn group relative grid size-6 place-items-center rounded-sm transition-[background-color,box-shadow,color] duration-150",
        "text-(--text-faint)",
        "enabled:cursor-pointer disabled:cursor-default disabled:opacity-40",
        "enabled:hover:text-(--text-h)",
        tone === "accent" && "enabled:hover:bg-(--accent-bg) enabled:hover:text-(--accent-deep)",
        tone === "danger" &&
          "enabled:hover:bg-[color-mix(in_srgb,#e0494a_14%,transparent)] enabled:hover:text-[#d23b3c]",
        "enabled:active:shadow-(--inset-lo)",
      )}
    >
      <span className="pointer-events-none transition-opacity duration-150 group-disabled:opacity-0">
        {children}
      </span>
    </button>
  );
}

function cx(...classes: Array<false | null | string | undefined>): string {
  return classes.filter(Boolean).join(" ");
}
