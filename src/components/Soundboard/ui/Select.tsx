import { ChevronDown, Check } from "lucide-react";
import {
  type ReactNode,
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { cx, formSelectClass } from "../styles";

interface Option {
  value: string;
  label: string;
  disabled?: boolean;
}

interface SelectProps {
  children: ReactNode;
  className?: string;
  disabled?: boolean;
  label: string;
  name?: string;
  value: string;
  onChange: (event: { target: { name?: string; value: string } }) => void;
}

interface MenuPosition {
  left: number;
  top: number;
  width: number;
  openUp: boolean;
}

/**
 * A stylised replacement for the native `<select>` element.
 *
 * It keeps the familiar controlled API (`value`, `onChange` receiving an event
 * with `target.value`, `<option>` children) while rendering a fully custom
 * button + portal-positioned dropdown that matches the studio-console theme.
 */
export function Select({
  children,
  className,
  disabled,
  label,
  name,
  value,
  onChange,
}: SelectProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const listId = useId();
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<MenuPosition | null>(null);
  const [highlight, setHighlight] = useState(-1);

  const options = parseOptions(children);
  const widestLabel = options.reduce<string>(
    (longest, option) => (option.label.length > longest.length ? option.label : longest),
    "",
  );
  const selectedLabel =
    options.find((option) => option.value === value)?.label ??
    (value.length > 0 ? value : (options[0]?.label ?? ""));

  const close = useCallback(() => setOpen(false), []);

  // Position the floating menu whenever it opens.
  useLayoutEffect(() => {
    if (!open || !triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    const menuHeight = 224; // matches max-h-56 (~14rem)
    const openUp = rect.bottom + menuHeight > window.innerHeight && rect.top - menuHeight > 8;
    setPosition({
      left: rect.left,
      top: openUp ? rect.top - menuHeight : rect.bottom,
      width: rect.width,
      openUp,
    });
  }, [open]);

  // Reset highlight when opening, and clamp it while navigating.
  useEffect(() => {
    if (!open) return;
    const initial = Math.max(
      0,
      options.findIndex((option) => option.value === value),
    );
    setHighlight(initial < 0 ? 0 : initial);
  }, [open, options, value]);

  // Close on outside click / Escape, and lock scroll while open.
  useEffect(() => {
    if (!open) return;
    const controller = new AbortController();

    document.addEventListener(
      "pointerdown",
      (event) => {
        const target = event.target as Node;
        if (triggerRef.current?.contains(target)) return;
        if (menuRef.current?.contains(target)) return;
        close();
      },
      { signal: controller.signal },
    );

    document.addEventListener(
      "keydown",
      (event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          close();
          triggerRef.current?.focus();
        }
      },
      { signal: controller.signal },
    );

    return () => controller.abort();
  }, [open, close]);

  // Keep the highlighted option scrolled into view.
  useEffect(() => {
    if (!open || highlight < 0) return;
    const item = listRef.current?.children.item(highlight) as HTMLElement | null;
    item?.scrollIntoView({ block: "nearest" });
  }, [open, highlight]);

  // Reposition on viewport changes while open.
  useEffect(() => {
    if (!open) return;
    const controller = new AbortController();
    window.addEventListener("resize", close, { signal: controller.signal });
    window.addEventListener("scroll", close, { capture: true, signal: controller.signal });
    return () => controller.abort();
  }, [open, close]);

  const choose = useCallback(
    (option: Option) => {
      if (option.disabled) return;
      onChange({ target: { name, value: option.value } });
      close();
      triggerRef.current?.focus();
    },
    [close, name, onChange],
  );

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (disabled) return;
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp":
      case "Enter":
      case " ":
      case "Home":
      case "End": {
        event.preventDefault();
        if (!open) {
          setOpen(true);
          return;
        }
        if (event.key === "Home") setHighlight(firstEnabled(options));
        else if (event.key === "End") setHighlight(lastEnabled(options));
        else if (event.key === "ArrowDown") setHighlight(nextEnabled(options, highlight, 1));
        else if (event.key === "ArrowUp") setHighlight(nextEnabled(options, highlight, -1));
        else if (event.key === "Enter" || event.key === " ") {
          const option = options[highlight];
          if (option) choose(option);
        }
        break;
      }
      default:
        break;
    }
  };

  return (
    <div className="relative inline-flex w-fit max-w-full">
      <span className="sr-only" id={`${listId}-label`}>
        {label}
      </span>
      <button
        ref={triggerRef}
        type="button"
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-labelledby={`${listId}-label`}
        className={cx(
          formSelectClass,
          "flex w-full items-center gap-2 pr-2 text-left",
          open && "border-(--accent-border)",
          className,
        )}
        onClick={() => !disabled && setOpen((v) => !v)}
        onKeyDown={onKeyDown}
      >
        <span className="relative block min-w-0 flex-1">
          {/* The widest option label, laid out invisibly in-flow, sizes the
              trigger to match a native <select>'s content width without
              adding extra height. */}
          <span aria-hidden="true" className="block truncate whitespace-nowrap opacity-0">
            {widestLabel || "—"}
          </span>
          <span
            className={cx(
              "absolute inset-0 truncate",
              value === "" && options.length > 0 && value !== options[0]?.value
                ? "text-(--text-faint)"
                : disabled && "opacity-55",
            )}
          >
            {selectedLabel || "—"}
          </span>
        </span>
        <ChevronDown
          size={15}
          aria-hidden="true"
          className={cx("shrink-0 text-(--text-faint) transition-transform", open && "rotate-180")}
        />
      </button>

      {open &&
        position &&
        createPortal(
          <div
            ref={menuRef}
            className="z-50 max-h-56 overflow-y-auto rounded-sm border border-(--border) bg-(--surface) py-1 shadow-(--shadow-lg) outline-none"
            style={{
              position: "fixed",
              left: position.left,
              top: position.top,
              width: position.width,
            }}
            role="listbox"
            aria-labelledby={`${listId}-label`}
          >
            <ul ref={listRef} className="flex flex-col">
              {options.map((option, index) => {
                const active = option.value === value;
                return (
                  <li
                    key={option.value}
                    role="option"
                    aria-selected={active}
                    aria-disabled={option.disabled}
                  >
                    <button
                      type="button"
                      disabled={option.disabled}
                      data-highlight={index === highlight}
                      onMouseEnter={() => setHighlight(index)}
                      onClick={() => choose(option)}
                      className={cx(
                        "flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-sm transition",
                        option.disabled
                          ? "cursor-not-allowed text-(--text-faint)"
                          : "cursor-pointer text-(--text-h)",
                        index === highlight &&
                          !option.disabled &&
                          "bg-(--accent-bg) text-(--accent)",
                        active && "font-semibold",
                      )}
                    >
                      <span className="truncate">{option.label}</span>
                      {active && (
                        <Check size={14} aria-hidden="true" className="shrink-0 text-(--accent)" />
                      )}
                    </button>
                  </li>
                );
              })}
              {options.length === 0 && <li className="px-3 py-2 text-sm text-(--text-faint)">—</li>}
            </ul>
          </div>,
          document.body,
        )}
    </div>
  );
}

function parseOptions(children: ReactNode): Option[] {
  const result: Option[] = [];
  for (const child of flat(children)) {
    if (
      child &&
      typeof child === "object" &&
      "props" in child &&
      child.props &&
      typeof child.props === "object" &&
      "value" in child.props
    ) {
      const props = child.props as {
        value?: unknown;
        disabled?: boolean;
        children?: ReactNode;
      };
      const raw = props.value;
      const value =
        typeof raw === "string" || typeof raw === "number" || typeof raw === "boolean"
          ? String(raw)
          : "";
      result.push({
        value,
        label: textOf(props.children) || value,
        disabled: Boolean(props.disabled),
      });
    }
  }
  return result;
}

function flat(children: ReactNode): ReactNode[] {
  if (Array.isArray(children)) return children.flatMap(flat);
  return [children];
}

function textOf(node: ReactNode): string {
  if (node == null || typeof node === "boolean") return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(textOf).join("");
  if (typeof node === "object" && "props" in node) {
    const props = (node as { props?: { children?: ReactNode } }).props;
    return textOf(props?.children);
  }
  return "";
}

function firstEnabled(options: Option[]): number {
  return options.findIndex((option) => !option.disabled);
}

function lastEnabled(options: Option[]): number {
  for (let i = options.length - 1; i >= 0; i--) if (!options[i].disabled) return i;
  return -1;
}

function nextEnabled(options: Option[], from: number, dir: 1 | -1): number {
  if (options.length === 0) return -1;
  let i = from < 0 ? (dir === 1 ? -1 : options.length) : from;
  for (let n = 0; n < options.length; n++) {
    i = (i + dir + options.length) % options.length;
    if (!options[i].disabled) return i;
  }
  return from < 0 ? 0 : from;
}
