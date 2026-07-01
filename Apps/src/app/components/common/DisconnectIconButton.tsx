import { Unplug } from "lucide-react";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";

type DisconnectIconButtonProps = {
  ariaLabel: string;
  disabled?: boolean;
  onClick: () => void;
  size?: "sm" | "md";
  title: string;
};

export function DisconnectIconButton({
  ariaLabel,
  disabled,
  onClick,
  size = "md",
  title,
}: DisconnectIconButtonProps) {
  const sizeClassName = size === "sm" ? "h-[22px] w-[22px] rounded-md" : "h-7 w-7 rounded-md";
  const iconSize = size === "sm" ? 13 : 14;

  return (
    <button
      type="button"
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={onClick}
      title={title}
      className={`inline-flex shrink-0 items-center justify-center bg-destructive/12 text-destructive ${SURFACE_REVEAL_BG} hover:bg-destructive/20 disabled:cursor-not-allowed disabled:opacity-40 ${sizeClassName}`}
    >
      <Unplug size={iconSize} aria-hidden="true" />
    </button>
  );
}
