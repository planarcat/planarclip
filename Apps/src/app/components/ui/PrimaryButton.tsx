import type { ButtonHTMLAttributes, ReactNode } from "react";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";

type PrimaryButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
};

export function PrimaryButton({ className = "", children, type = "button", ...rest }: PrimaryButtonProps) {
  return (
    <button
      type={type}
      className={`rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground ${SURFACE_REVEAL_BG} hover:bg-[var(--button-primary-hover-bg)] disabled:cursor-not-allowed disabled:opacity-40 ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
