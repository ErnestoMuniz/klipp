import { createContext, useContext, useCallback, useMemo, useState } from "react";
import { ptBR as ptBRMessages } from "./locales/pt-BR";
import { en, type Messages } from "./locales/en";

/** A concrete, fully-resolved locale the UI can render. */
export type Locale = "en" | "pt-BR";

/**
 * The user-facing language preference.
 * - `"system"` follows the OS preferred language, falling back to English.
 * - Any other value pins the UI to that locale regardless of the OS.
 */
export type LanguagePref = Locale | "system";

export const LANGUAGE_STORAGE_KEY = "klipp.locale.v1";
export const SUPPORTED_LOCALES: Locale[] = ["en", "pt-BR"];

export type TranslationKey = keyof Messages;

export interface I18nContextValue {
  /** The user's stored preference (including `"system"`). */
  language: LanguagePref;
  /** The concrete locale currently rendered. */
  locale: Locale;
  /** Set the stored preference; persists and re-resolves the locale. */
  setLanguage: (language: LanguagePref) => void;
  /** Translate a key, optionally interpolating `{placeholder}` values. */
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}

const catalogs: Record<Locale, Messages> = {
  en,
  "pt-BR": ptBRMessages,
};

/** Map a BCP-47 language tag to one of our supported locales, or `null`. */
function localeFromTag(tag: string): Locale | null {
  if (!tag) return null;
  const primary = tag.toLowerCase().split("-")[0];
  // Brazilian Portuguese is the only Portuguese variant we ship, so any
  // Portuguese primary subtag resolves to it.
  return primary === "pt" ? "pt-BR" : null;
}

/**
 * Detect the preferred locale from the browser/OS language settings.
 * Walks `navigator.languages` in priority order and returns the first match,
 * falling back to English (the app's main language) when nothing matches.
 */
export function detectLocale(): Locale {
  const languages =
    typeof navigator !== "undefined" && navigator.languages?.length
      ? navigator.languages
      : typeof navigator !== "undefined"
        ? [navigator.language]
        : [];

  for (const tag of languages) {
    const match = localeFromTag(tag);
    if (match) return match;
  }
  return "en";
}

/** Resolve a stored preference into a concrete locale. */
export function resolveLocale(pref: LanguagePref): Locale {
  return pref === "system" ? detectLocale() : pref;
}

/** Read the persisted language preference, defaulting to “follow system”. */
export function loadLanguage(): LanguagePref {
  try {
    const raw = localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (raw === "en" || raw === "pt-BR" || raw === "system") return raw;
  } catch {
    /* ignore storage errors */
  }
  return "system";
}

function saveLanguage(pref: LanguagePref): void {
  try {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, pref);
  } catch {
    /* ignore storage errors */
  }
}

export const I18nContext = createContext<I18nContextValue | null>(null);

/**
 * React state backing the i18n provider. Kept in a non-component module so the
 * provider file can export only a component (required by the
 * `react/only-export-components` lint rule for fast-refresh).
 */
export function useI18nState(initialLanguage?: LanguagePref): I18nContextValue {
  const [language, setLanguageState] = useState<LanguagePref>(initialLanguage ?? loadLanguage);
  const locale = useMemo(() => resolveLocale(language), [language]);

  const setLanguage = useCallback((next: LanguagePref) => {
    setLanguageState(next);
    saveLanguage(next);
  }, []);

  const t = useCallback(
    (key: TranslationKey, params?: Record<string, string | number>) => {
      const catalog = catalogs[locale] ?? en;
      let message: string = catalog[key] ?? en[key] ?? key;
      if (params) {
        for (const [name, value] of Object.entries(params)) {
          message = message.replaceAll(`{${name}}`, String(value));
        }
      }
      return message;
    },
    [locale],
  );

  return useMemo<I18nContextValue>(
    () => ({ language, locale, setLanguage, t }),
    [language, locale, setLanguage, t],
  );
}

/** Access the current i18n context. Throws if used outside `I18nProvider`. */
export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within an I18nProvider");
  return ctx;
}
