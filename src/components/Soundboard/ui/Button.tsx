import type { ButtonHTMLAttributes, ReactNode } from "react";
import { activeIconButtonClass, cx, iconButtonClass, primaryButtonClass } from "../styles";

type ButtonVariant = "primary" | "secondary" | "ghost" | "icon" | "transport" | "quietIcon";
type ButtonSize = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  active?: boolean;
  children: ReactNode;
  size?: ButtonSize;
  variant?: ButtonVariant;
}

const baseButtonClass = "transition disabled:cursor-not-allowed";

const ghostButtonClass =
  "cursor-pointer rounded-sm border border-transparent bg-transparent px-3 py-2 text-sm font-medium text-(--text) hover:bg-(--surface-sunk) hover:text-(--text-h) aria-pressed:text-(--accent)";

const secondaryButtonClass =
  "cursor-pointer rounded-sm border border-(--border-strong) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] px-3 py-2 font-medium text-(--text-h) shadow-[var(--shadow-sm),var(--inset-hi)] hover:border-(--accent-border) hover:text-(--accent) disabled:opacity-55";

const quietIconButtonClass =
  "grid shrink-0 cursor-pointer place-items-center border border-(--border) bg-(--surface-sunk) text-(--text-h) shadow-(--inset-lo) hover:border-(--accent-border) hover:text-(--accent)";

const transportButtonClass =
  "grid shrink-0 place-items-center border border-(--border) bg-(--surface-sunk) text-(--text-faint) shadow-(--inset-lo) disabled:cursor-default disabled:opacity-60 enabled:cursor-pointer enabled:border-(--accent-deep) enabled:bg-[linear-gradient(180deg,var(--accent),var(--accent-deep))] enabled:text-(--accent-ink) enabled:shadow-[0_6px_18px_-8px_var(--accent-glow),inset_0_1px_0_rgba(255,255,255,0.25)] enabled:hover:bg-[linear-gradient(180deg,color-mix(in_srgb,var(--accent)_92%,#fff_8%),var(--accent-deep))] enabled:active:translate-y-px";

const iconSizes: Record<ButtonSize, string> = {
  sm: "size-9 rounded-sm",
  md: "size-10 rounded",
  lg: "size-11 rounded-xl",
};

export function Button({
  active = false,
  children,
  className,
  size = "md",
  type = "button",
  variant = "primary",
  ...props
}: ButtonProps) {
  const variantClass = {
    primary: primaryButtonClass,
    secondary: secondaryButtonClass,
    ghost: ghostButtonClass,
    icon: cx(iconButtonClass, active && activeIconButtonClass),
    quietIcon: cx(quietIconButtonClass, iconSizes[size]),
    transport: cx(transportButtonClass, iconSizes[size]),
  }[variant];

  return (
    <button type={type} className={cx(baseButtonClass, variantClass, className)} {...props}>
      {children}
    </button>
  );
}
