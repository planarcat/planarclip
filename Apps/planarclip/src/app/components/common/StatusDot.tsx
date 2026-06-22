import type { DeviceStatus } from "../../types";

type StatusDotProps = {
  status: DeviceStatus;
};

export function StatusDot({ status }: StatusDotProps) {
  const className =
    status === "connected"
      ? "bg-emerald-400 shadow-[0_0_6px_#34d399]"
      : status === "idle"
        ? "bg-amber-400 shadow-[0_0_6px_#fbbf24]"
        : "bg-zinc-600";

  return <span className={`inline-block h-1.5 w-1.5 shrink-0 rounded-full ${className}`} />;
}
