import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../styles";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  tone?: "raised" | "sunken" | "dashed" | "alert";
}

export function Card({ children, className, tone = "raised", ...props }: CardProps) {
  const toneClass = {
    raised:
      "rounded-lg border border-(--border) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] shadow-[var(--shadow-sm),var(--inset-hi)]",
    sunken: "rounded border border-(--border) bg-(--surface-sunk) shadow-(--inset-lo)",
    dashed:
      "rounded-lg border border-dashed border-(--border-strong) bg-(--surface-sunk) shadow-(--inset-lo)",
    alert:
      "rounded-lg border border-(--accent-border) bg-[linear-gradient(180deg,color-mix(in_srgb,var(--accent)_8%,var(--surface)),var(--surface))] shadow-[var(--shadow-sm),var(--inset-hi)]",
  }[tone];

  return (
    <div className={cx(toneClass, className)} {...props}>
      {children}
    </div>
  );
}
