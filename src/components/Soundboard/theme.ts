import type { Theme } from "./types";

/**
 * Applies the selected theme to the document root.
 * - "light": forces light mode (adds .light, removes .dark)
 * - "dark": forces dark mode (adds .dark, removes .light)
 * - "system": follows OS preference (removes both .light and .dark)
 */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  root.classList.remove("light", "dark");

  switch (theme) {
    case "light":
      root.classList.add("light");
      root.style.colorScheme = "light";
      break;
    case "dark":
      root.classList.add("dark");
      root.style.colorScheme = "dark";
      break;
    case "system":
      root.style.colorScheme = "light dark";
      break;
  }
}

/**
 * Resolves the effective theme based on user preference and system preference.
 * Useful for displaying the active mode to the user.
 */
export function resolveTheme(theme: Theme): "light" | "dark" {
  if (theme === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return theme;
}
