import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../styles";

interface IconTileProps extends HTMLAttributes<HTMLSpanElement> {
  children: ReactNode;
  size?: "sm" | "md" | "lg";
}

const sizes = {
  sm: "size-6 rounded-md",
  md: "size-7 rounded-sm",
  lg: "size-14 rounded-2xl",
};

export function IconTile({ children, className, size = "md", ...props }: IconTileProps) {
  return (
    <span
      className={cx(
        "grid shrink-0 place-items-center bg-[linear-gradient(160deg,var(--ink-soft),var(--ink))] text-(--accent) shadow-[var(--shadow-sm),var(--inset-hi)]",
        size === "sm" && "bg-(--ink) shadow-none",
        sizes[size],
        className,
      )}
      {...props}
    >
      {children}
    </span>
  );
}
