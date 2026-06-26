import { BrandLogo } from "./BrandLogo";
import { TitleBarButtons } from "./TitleBarButtons";

export function Topbar() {
  return (
    <header className="app-drag sticky top-0 z-8 -mx-6 flex items-center justify-between border-b border-(--hairline) bg-[linear-gradient(180deg,color-mix(in_srgb,var(--bg)_92%,transparent),color-mix(in_srgb,var(--bg)_78%,transparent))] px-1 py-1 backdrop-blur-[14px] backdrop-saturate-[1.1]">
      <div className="inline-flex shrink-0 items-center gap-2 pl-1">
        <BrandLogo />
        <span className="hidden text-lg font-semibold sm:inline text-transparent bg-clip-text bg-[linear-gradient(180deg,var(--accent),var(--accent-deep))]">
          Klipp
        </span>
      </div>

      <TitleBarButtons />
    </header>
  );
}
