import { BrandLogo } from "./BrandLogo";
import { TitleBarButtons } from "./TitleBarButtons";

export function Topbar() {
  return (
    <header className="app-drag sticky top-0 z-8 -mx-3.5 flex items-center justify-between border-b border-(--hairline) bg-[linear-gradient(180deg,color-mix(in_srgb,var(--bg)_92%,transparent),color-mix(in_srgb,var(--bg)_78%,transparent))] px-3.5 py-2 backdrop-blur-[14px] backdrop-saturate-[1.1]">
      <div className="inline-flex shrink-0 items-center gap-2.5">
        <BrandLogo />
        <span className="hidden text-base font-semibold text-(--text-h) sm:inline">Klipp</span>
      </div>

      <TitleBarButtons />
    </header>
  );
}
