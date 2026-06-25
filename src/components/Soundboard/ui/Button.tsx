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
  "cursor-pointer rounded-sm border border-transparent bg-transparent px-3 py-2 text-sm font-medium text-[var(--text)] hover:bg-[var(--surface-sunk)] hover:text-[var(--text-h)] aria-pressed:text-[var(--accent)]";

const secondaryButtonClass =
  "cursor-pointer rounded-sm border border-[var(--border-strong)] bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] px-3 py-2 font-medium text-[var(--text-h)] shadow-[var(--shadow-sm),var(--inset-hi)] hover:border-[var(--accent-border)] hover:text-[var(--accent)] disabled:opacity-55";

const quietIconButtonClass =
  "grid shrink-0 cursor-pointer place-items-center border border-[var(--border)] bg-[var(--surface-sunk)] text-[var(--text-h)] shadow-[var(--inset-lo)] hover:border-[var(--accent-border)] hover:text-[var(--accent)]";

const transportButtonClass =
  "grid shrink-0 place-items-center border border-[var(--border)] bg-[var(--surface-sunk)] text-[var(--text-faint)] shadow-[var(--inset-lo)] disabled:cursor-default disabled:opacity-60 enabled:cursor-pointer enabled:border-[var(--accent-deep)] enabled:bg-[linear-gradient(180deg,var(--accent),var(--accent-deep))] enabled:text-[var(--accent-ink)] enabled:shadow-[0_6px_18px_-8px_var(--accent-glow),inset_0_1px_0_rgba(255,255,255,0.25)] enabled:hover:bg-[linear-gradient(180deg,color-mix(in_srgb,var(--accent)_92%,#fff_8%),var(--accent-deep))] enabled:active:translate-y-px";

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
