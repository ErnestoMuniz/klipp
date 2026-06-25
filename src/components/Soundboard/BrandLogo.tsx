import { logoClass } from "./styles";

interface BrandLogoProps {
  large?: boolean;
}

export function BrandLogo({ large = false }: BrandLogoProps) {
  return (
    <div
      className={large ? `${logoClass} size-14 rounded-2xl text-2xl` : logoClass}
      aria-hidden="true"
    >
      K
    </div>
  );
}
