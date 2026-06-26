import logo from "../../assets/logo.webp";
import { logoClass } from "./styles";

interface BrandLogoProps {
  large?: boolean;
}

export function BrandLogo({ large = false }: BrandLogoProps) {
  return (
    <div className={large ? `${logoClass} size-14 rounded-2xl` : logoClass} aria-hidden="true">
      <img src={logo} alt="" className="size-full" />
    </div>
  );
}
