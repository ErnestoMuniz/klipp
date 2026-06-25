import { cx, eqBarClass } from "./styles";

interface EqualizerProps {
  live?: boolean;
  transport?: boolean;
}

export function Equalizer({ live = true, transport = false }: EqualizerProps) {
  const barColor = transport ? (live ? "bg-(--accent)" : "bg-(--text-faint)") : "bg-(--accent-ink)";

  return (
    <span
      className={cx(
        "inline-flex shrink-0 items-end gap-1",
        transport ? "h-5.5 w-6.5" : "h-4.5",
        transport && (live ? "opacity-100" : "opacity-30"),
      )}
      aria-hidden="true"
    >
      <span className={cx(eqBarClass, "h-1.25 flex-1", barColor)} />
      <span className={cx(eqBarClass, "h-1.25 flex-1 [animation-delay:0.18s]", barColor)} />
      <span className={cx(eqBarClass, "h-1.25 flex-1 [animation-delay:0.36s]", barColor)} />
      <span className={cx(eqBarClass, "h-1.25 flex-1 [animation-delay:0.12s]", barColor)} />
    </span>
  );
}
