import { useEffect, useState } from "react";

/**
 * Custom macOS-style traffic-light window controls (close / minimize /
 * maximize). Rendered inside the (frameless) main window's Topbar. The
 * buttons are left-aligned, like native macOS traffic lights.
 *
 * The whole group opts out of the Topbar's drag region so each button stays
 * clickable.
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
    <div className="app-no-drag flex shrink-0 items-center gap-2" aria-label="Controles da janela">
      <TrafficLight
        label="Minimizar"
        color="minimize"
        disabled={!available}
        onClick={() => controls?.minimize()}
      >
        <MinimizeGlyph />
      </TrafficLight>
      <TrafficLight
        label={maximized ? "Restaurar" : "Maximizar"}
        color="maximize"
        disabled={!available}
        onClick={() => controls?.toggleMaximize()}
      >
        <MaximizeGlyph maximized={maximized} />
      </TrafficLight>
      <TrafficLight
        label="Fechar"
        color="close"
        disabled={!available}
        onClick={() => controls?.close()}
      >
        <CloseGlyph />
      </TrafficLight>
    </div>
  );
}

type TrafficColor = "close" | "minimize" | "maximize";

interface TrafficLightProps {
  label: string;
  color: TrafficColor;
  disabled?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}

function TrafficLight({ label, color, disabled, onClick, children }: TrafficLightProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className={cx(
        "tl group grid size-3.5 place-items-center rounded-full border border-black/15 transition-all duration-150",
        "enabled:cursor-pointer enabled:hover:scale-105 disabled:cursor-default",
        color === "close" && "tl-close",
        color === "minimize" && "tl-minimize",
        color === "maximize" && "tl-maximize",
      )}
    >
      <span className="tl-glyph pointer-events-none opacity-0 transition-opacity duration-150 group-hover:opacity-100">
        {children}
      </span>
    </button>
  );
}

function CloseGlyph() {
  return (
    <svg viewBox="0 0 10 10" className="size-2" aria-hidden="true">
      <path
        d="M2 2 L8 8 M8 2 L2 8"
        className="tl-stroke"
        fill="none"
        strokeWidth="1.4"
        strokeLinecap="round"
      />
    </svg>
  );
}

function MinimizeGlyph() {
  return (
    <svg viewBox="0 0 10 10" className="size-2" aria-hidden="true">
      <path d="M2 5 H8" className="tl-stroke" fill="none" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  );
}

function MaximizeGlyph({ maximized }: { maximized: boolean }) {
  if (maximized) {
    // two overlapping rectangles (restore)
    return (
      <svg viewBox="0 0 10 10" className="size-2" aria-hidden="true">
        <rect
          x="2.4"
          y="1.6"
          width="4.4"
          height="4.4"
          rx="0.6"
          className="tl-stroke"
          fill="none"
          strokeWidth="1.2"
        />
        <rect
          x="3.6"
          y="3.2"
          width="4.4"
          height="4.4"
          rx="0.6"
          className="tl-stroke"
          fill="none"
          strokeWidth="1.2"
        />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 10 10" className="size-2" aria-hidden="true">
      <path
        d="M2.6 2 L7.4 2 L7.4 7 L4.6 7 L2.6 5 Z"
        className="tl-stroke"
        fill="none"
        strokeWidth="1.2"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function cx(...classes: Array<false | null | string | undefined>): string {
  return classes.filter(Boolean).join(" ");
}
