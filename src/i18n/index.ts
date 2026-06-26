// Barrel for the i18n module. Re-exporting only (no local declarations)
// keeps fast-refresh happy: `react/only-export-components` ignores pure
// re-export files. Components and helpers live in `./core`; the provider
// component lives in `./I18nProvider`.
export * from "./core";
export { I18nProvider } from "./I18nProvider";
