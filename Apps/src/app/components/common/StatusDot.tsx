import type { DeviceStatus } from "../../types";

type StatusDotProps = {
  status: DeviceStatus;
  size?: "sm" | "md" | "lg";
};

function getStatusLabel(status: DeviceStatus) {
  return status === "connected" ? "已连接" : status === "idle" ? "可连接" : "离线";
}

export function StatusDot({ status, size = "md" }: StatusDotProps) {
  const colorClassName =
    status === "connected"
      ? "bg-emerald-400 shadow-[0_0_6px_#34d399]"
      : status === "idle"
        ? "bg-amber-400 shadow-[0_0_6px_#fbbf24]"
        : "bg-zinc-500 shadow-[0_0_6px_rgba(113,113,122,0.35)]";

  const sizeClassName = size === "lg" ? "h-3 w-3" : size === "sm" ? "h-2 w-2" : "h-2.5 w-2.5";
  const label = getStatusLabel(status);

  return (
    <span
      aria-label={label}
      className={`inline-block shrink-0 rounded-full ${sizeClassName} ${colorClassName}`}
      role="img"
      title={label}
    />
  );
}
