import { type ReactNode } from "react";
import { I18nContext, useI18nState, type LanguagePref } from "./core";

/**
 * Provides i18n state to the React tree.
 *
 * Both the main soundboard window and the quick-picker overlay window render
 * this provider, so neither needs to share state across windows — each reads
 * the persisted preference synchronously at mount.
 */
export function I18nProvider({
  children,
  initialLanguage,
}: {
  children: ReactNode;
  initialLanguage?: LanguagePref;
}) {
  const value = useI18nState(initialLanguage);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}
