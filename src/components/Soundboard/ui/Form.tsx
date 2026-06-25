import type { InputHTMLAttributes, LabelHTMLAttributes, ReactNode } from "react";
import { cx } from "../styles";
interface SwitchProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  label: string;
}

export function Switch({ checked, disabled, label, onChange, ...props }: SwitchProps) {
  return (
    <label className="flex cursor-pointer items-center gap-3 text-sm leading-normal text-(--text-h)">
      <span className="relative inline-block h-6.5 w-11 shrink-0">
        <input
          type="checkbox"
          className="peer absolute inset-0 m-0 cursor-pointer opacity-0"
          checked={checked}
          disabled={disabled}
          onChange={onChange}
          {...props}
        />
        <span className="absolute inset-0 rounded-full border border-(--border) bg-(--surface-sunk) shadow-(--inset-lo) transition peer-checked:border-(--accent-deep) peer-checked:bg-[linear-gradient(180deg,var(--accent),var(--accent-deep))] peer-disabled:opacity-50 after:absolute after:left-1 after:top-1 after:size-4.5 after:rounded-full after:border after:border-(--border-strong) after:bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] after:shadow-(--shadow-sm) after:transition-transform peer-checked:after:translate-x-4.5 peer-checked:after:border-(--accent-deep)" />
      </span>
      <span className="flex-1">{label}</span>
    </label>
  );
}

interface FieldGroupProps extends LabelHTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  label: string;
}

export function FieldGroup({ children, className, label, ...props }: FieldGroupProps) {
  return (
    <div className={cx("flex flex-col gap-3", className)} {...props}>
      <div className="font-mono text-xs font-semibold uppercase tracking-widest text-(--text-faint)">
        {label}
      </div>
      {children}
    </div>
  );
}
