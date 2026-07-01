import type { ButtonHTMLAttributes, ReactNode } from "react";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";

type IconButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
  size?: "md" | "sm";
};

export function IconButton({ className = "", children, size = "md", type = "button", ...rest }: IconButtonProps) {
  const sizeClass = size === "sm" ? "h-7 w-7 rounded-md p-0" : "rounded-lg p-1.5";

  return (
    <button
      type={type}
      className={`inline-flex shrink-0 items-center justify-center text-secondary-foreground ${SURFACE_REVEAL_BG} hover:bg-secondary hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40 ${sizeClass} ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
