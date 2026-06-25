import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../styles";

interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  children: ReactNode;
  tone?: "accent" | "neutral" | "status";
}

export function Badge({ children, className, tone = "accent", ...props }: BadgeProps) {
  const toneClass = {
    accent: "border-(--accent-border) bg-(--accent-bg) text-(--accent)",
    neutral: "border-(--border) bg-(--surface-sunk) text-(--text)",
    status: "border-(--border) bg-(--surface-sunk) text-(--text)",
  }[tone];

  return (
    <span
      className={cx(
        "inline-flex shrink-0 items-center gap-2 rounded-full border px-2 py-px font-mono text-xs font-semibold tabular-nums tracking-wide",
        toneClass,
        className,
      )}
      {...props}
    >
      {children}
    </span>
  );
}
