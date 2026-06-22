import { useRelativeTicker } from "../../hooks/useRelativeTicker";
import type { AppConnectionStatus, Device } from "../../types";
import { relativeTime } from "../../utils/time";
import { OsIcon } from "../common/OsIcon";
import { StatusDot } from "../common/StatusDot";

type DevicesPanelProps = {
  devices: Device[];
  pairingCode: string;
  status: AppConnectionStatus;
};

export function DevicesPanel({ devices, pairingCode, status }: DevicesPanelProps) {
  useRelativeTicker();

  return (
    <aside className="h-full w-60 shrink-0 overflow-y-auto border-l border-border bg-card xl:w-64">
      <div className="border-b border-border px-4 pb-3 pt-5">
        <p className="text-sm font-semibold text-foreground">连接概览</p>
        <p className="mt-0.5 text-xs text-muted-foreground">局域网发现与直连状态</p>
      </div>
      <div className="space-y-2 p-3">
        {devices.length > 0 ? (
          devices.map((device) => (
            <div key={device.id} className="rounded-lg border border-border bg-secondary/30 p-3 transition-colors hover:border-primary/30">
              <div className="mb-2 flex items-start justify-between">
                <div className="flex items-center gap-2">
                  <div className="rounded bg-primary/10 p-1.5 text-primary">
                    <OsIcon os={device.os} size={14} />
                  </div>
                  <div>
                    <p className="text-xs font-medium leading-none text-foreground">{device.name}</p>
                    <p className="mt-0.5 font-mono text-[10px] text-muted-foreground">
                      {device.os === "macos" ? "macOS 系统" : "Windows 系统"}
                    </p>
                  </div>
                </div>
                <StatusDot status={device.status} />
              </div>
              <div className="space-y-1">
                {[
                  { label: "连接地址", value: device.address, className: "text-foreground/80" },
                  {
                    label: "状态",
                    value: device.status === "connected" ? "已连接" : device.source === "discovery" ? "已发现" : "离线",
                    className: device.status === "connected" ? "text-emerald-400" : "text-amber-400",
                  },
                  { label: "最近活跃", value: relativeTime(device.lastSeen), className: "text-foreground/70" },
                ].map((row) => (
                  <div key={row.label} className="flex items-center justify-between gap-3">
                    <span className="text-[10px] text-muted-foreground">{row.label}</span>
                    <span className={`truncate font-mono text-[10px] ${row.className}`}>{row.value}</span>
                  </div>
                ))}
              </div>
            </div>
          ))
        ) : (
          <div className="rounded-lg border border-dashed border-border px-3 py-6 text-center text-xs text-muted-foreground">
            还没有发现附近设备
          </div>
        )}
      </div>
      <div className="mx-3 mb-3 rounded-lg border border-border bg-primary/5 p-3">
        <p className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">网络信息</p>
        <div className="space-y-1">
          {[
            { label: "配对码", value: pairingCode, className: "text-primary" },
            {
              label: "连接状态",
              value: status === "online" ? "已连接" : status === "connecting" ? "连接中" : "等待连接",
              className: status === "online" ? "text-emerald-400" : status === "connecting" ? "text-amber-400" : "text-foreground/80",
            },
            { label: "发现设备", value: `${devices.length} 台`, className: "text-foreground/80" },
            { label: "加密", value: "AES-256-GCM", className: "text-emerald-400" },
          ].map((row) => (
            <div key={row.label} className="flex justify-between gap-3">
              <span className="text-[10px] text-muted-foreground">{row.label}</span>
              <span className={`font-mono text-[10px] ${row.className}`}>{row.value}</span>
            </div>
          ))}
        </div>
      </div>
    </aside>
  );
}
