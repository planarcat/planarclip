import { Unplug } from "lucide-react";

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
  const sizeClassName = size === "sm" ? "h-[22px] w-[22px] rounded-md" : "h-[26px] w-[26px] rounded-[5.5px]";
  const iconSize = size === "sm" ? 13 : 14;

  return (
    <button
      type="button"
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={onClick}
      title={title}
      className={`inline-flex shrink-0 items-center justify-center bg-destructive/12 text-destructive transition-colors hover:bg-destructive/20 disabled:cursor-not-allowed disabled:opacity-40 ${sizeClassName}`}
    >
      <Unplug size={iconSize} aria-hidden="true" />
    </button>
  );
}
