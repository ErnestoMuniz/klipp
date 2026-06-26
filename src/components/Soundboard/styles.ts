export function cx(...classes: Array<false | null | string | undefined>): string {
  return classes.filter(Boolean).join(" ");
}

export const logoClass =
  "relative grid size-10 shrink-0 place-items-center rounded-xl bg-[linear-gradient(160deg,var(--border),var(--surface-raise))] font-sans text-lg font-semibold text-(--ink-soft) shadow-[var(--shadow-sm),inset_0_1px_0_rgba(255,255,255,0.08),inset_0_-2px_4px_rgba(0,0,0,0.3)] after:absolute after:right-1 after:top-1 after:size-1.5 after:rounded-full after:bg-(--accent) after:shadow-[0_0_6px_1px_var(--accent-glow)]";

export const iconButtonClass =
  "grid size-10 shrink-0 place-items-center rounded border border-(--border) bg-[linear-gradient(180deg,var(--surface-raise),var(--surface))] text-(--text-h) shadow-[var(--shadow-sm),var(--inset-hi)] transition-[border-color,color,box-shadow] duration-150 hover:border-(--accent-border) hover:text-(--accent)";

export const activeIconButtonClass =
  "border-(--accent) bg-(--accent-bg) text-(--accent) shadow-[0_0_0_3px_var(--accent-bg),var(--inset-hi)]";

export const primaryButtonClass =
  "inline-flex items-center gap-1.5 rounded-md border border-(--accent-deep) bg-[linear-gradient(180deg,var(--accent),var(--accent-deep))] px-4 py-2 text-sm font-semibold text-(--accent-ink) transition active:translate-y-px disabled:cursor-not-allowed disabled:opacity-45 disabled:shadow-none enabled:cursor-pointer enabled:hover:bg-[linear-gradient(180deg,color-mix(in_srgb,var(--accent)_92%,#fff_8%),var(--accent-deep))]";

export const dividerClass =
  "h-px bg-[linear-gradient(90deg,transparent,var(--border),transparent)]";

export const formSelectClass =
  "cursor-pointer rounded-md border border-(--border) bg-(--surface-sunk) px-3 py-2.5 text-sm text-(--text-h) shadow-(--inset-lo) hover:not-disabled:border-(--accent-border) disabled:cursor-not-allowed disabled:opacity-55";

export const eqBarClass =
  "w-1 animate-[sb-eq-bar_0.8s_ease-in-out_infinite] rounded-sm motion-reduce:animate-none";
