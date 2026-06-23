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
        <p className="text-sm font-semibold text-primary">连接概览</p>
        <p className="mt-0.5 text-[13px] font-medium text-muted-foreground">局域网发现与直连状态</p>
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
                    <p className="text-[13px] font-medium leading-none text-primary">{device.name}</p>
                    <p className="mt-0.5 text-[13px] font-medium text-muted-foreground">
                      {device.hostName ? `主机名 ${device.hostName}` : device.os === "macos" ? "macOS 系统" : "Windows 系统"}
                    </p>
                  </div>
                </div>
                <StatusDot status={device.status} size="lg" />
              </div>
              <div className="space-y-1.5">
                {[
                  { label: "连接地址", value: device.address, className: "text-secondary-foreground", mono: true },
                  { label: "主机名", value: device.hostName ?? "暂未提供", className: "text-secondary-foreground", mono: false },
                  { label: "最近活跃", value: relativeTime(device.lastSeen), className: "text-secondary-foreground", mono: false },
                ].map((row) => (
                  <div key={row.label} className="flex items-center justify-between gap-3 leading-5">
                    <span className="text-[13px] font-medium text-muted-foreground">{row.label}</span>
                    <span className={`truncate text-[13px] font-medium ${row.mono ? "font-mono" : ""} ${row.className}`}>{row.value}</span>
                  </div>
                ))}
                <div className="flex items-center justify-between gap-3 leading-5">
                  <span className="text-[13px] font-medium text-muted-foreground">状态</span>
                  <StatusDot status={device.status} size="md" />
                </div>
              </div>
            </div>
          ))
        ) : (
          <div className="rounded-lg border border-dashed border-border px-3 py-6 text-center text-[13px] font-medium text-muted-foreground">
            还没有发现附近设备
          </div>
        )}
      </div>
      <div className="mx-3 mb-3 rounded-lg border border-border bg-primary/5 p-3">
        <p className="mb-2 text-[13px] font-medium text-primary">网络信息</p>
        <div className="space-y-1.5">
          {[
            { label: "配对码", value: pairingCode, className: "text-primary", mono: true },
            {
              label: "连接状态",
              value: status === "online" ? "已连接" : status === "connecting" ? "连接中" : "等待连接",
              className: status === "online" ? "text-emerald-500" : status === "connecting" ? "text-amber-500" : "text-secondary-foreground",
              mono: false,
            },
            { label: "发现设备", value: `${devices.length} 台`, className: "text-secondary-foreground", mono: false },
            { label: "加密", value: "AES-256-GCM", className: "text-emerald-500", mono: true },
          ].map((row) => (
            <div key={row.label} className="flex justify-between gap-3 leading-5">
              <span className="text-[13px] font-medium text-muted-foreground">{row.label}</span>
              <span className={`text-[13px] font-medium ${row.mono ? "font-mono" : ""} ${row.className}`}>{row.value}</span>
            </div>
          ))}
        </div>
      </div>
    </aside>
  );
}
