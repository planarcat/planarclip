import type { SettingAvailability } from "../../types";

type SettingBadgeProps = {
  availability: SettingAvailability;
};

export function SettingBadge({ availability }: SettingBadgeProps) {
  if (availability === "editable") {
    return null;
  }

  const config =
    availability === "managed"
      ? {
          label: "系统内置",
          className: "border-emerald-400/30 bg-emerald-400/10 text-emerald-400",
        }
      : {
          label: "暂不支持",
          className: "border-border bg-secondary text-muted-foreground",
        };

  return <span className={`shrink-0 rounded-full border px-2.5 py-1 text-xs font-medium ${config.className}`}>{config.label}</span>;
}
