import { Zap } from "lucide-react";
import { useMemo } from "react";
import { MAX_CONNECTIONS } from "../../constants/connection";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";
import { useRelativeTicker } from "../../hooks/useRelativeTicker";
import type { AppConnectionStatus, Device } from "../../types";
import { relativeTime } from "../../utils/time";
import { DisconnectIconButton } from "../common/DisconnectIconButton";
import { OsIcon } from "../common/OsIcon";
import { ScrollArea } from "../ui/ScrollArea";

type DevicesPanelProps = {
  devices: Device[];
  connectionStatus: AppConnectionStatus;
  connectionLocked: boolean;
  connectedCount: number;
  onConnectDevice: (device: Device) => void;
  onDisconnect: (device: Device) => void;
};

function buildConnectAction(device: Device, busyConnecting: boolean, atConnectionLimit: boolean) {
  const connectDisabled = busyConnecting || atConnectionLimit || !device.host || !device.port;
  const connectTitle = !device.host || !device.port
    ? "等待对方上线后再连接"
    : busyConnecting
      ? "正在处理连接，请稍候"
      : atConnectionLimit
        ? "已超出连接上限，请先断开其中一个设备"
        : `连接到 ${device.name}`;

  return {
    ariaLabel: connectTitle,
    disabled: connectDisabled,
    title: connectTitle,
  };
}

export function DevicesPanel({
  devices,
  connectionStatus,
  connectionLocked,
  connectedCount,
  onConnectDevice,
  onDisconnect,
}: DevicesPanelProps) {
  useRelativeTicker();

  const busyConnecting = connectionStatus === "connecting" || connectionLocked;
  const atConnectionLimit = connectedCount >= MAX_CONNECTIONS;
  const onlineDevices = useMemo(() => devices.filter((device) => device.status !== "offline"), [devices]);

  return (
    <ScrollArea as="aside" className="h-full w-60 shrink-0 overflow-y-auto border-l border-border bg-card xl:w-64">
      <div className="border-b border-border px-4 pb-3 pt-5">
        <p className="text-sm font-semibold text-primary">连接概览</p>
        <p className="mt-0.5 text-[13px] font-medium text-muted-foreground">局域网发现与直连状态</p>
      </div>
      <div className="space-y-2 p-3">
        {onlineDevices.length > 0 ? (
          onlineDevices.map((device) => {
            const hostNameLabel = device.hostName?.trim();
            const showHostName = Boolean(hostNameLabel && hostNameLabel.toLocaleLowerCase() !== device.name.trim().toLocaleLowerCase());
            const osLabel = device.os === "macos" ? "macOS" : "Windows";
            const isConnected = device.status === "connected";
            const disconnectTitle = `断开与 ${device.name} 的连接`;
            const connectAction = buildConnectAction(device, busyConnecting, atConnectionLimit);

            return (
              <div key={device.id} className="rounded-lg border border-border bg-secondary/30 p-3 transition-colors hover:border-primary/30">
                <div className="mb-2 flex items-start justify-between gap-2">
                  <div className="flex min-w-0 items-center gap-2">
                    <div
                      className={`rounded p-1.5 ${
                        isConnected ? "bg-primary/10 text-primary" : "bg-secondary text-muted-foreground"
                      }`}
                    >
                      <OsIcon os={device.os} size={14} />
                    </div>
                    <div className="min-w-0">
                      <p
                        className={`truncate text-[13px] font-medium leading-none ${
                          isConnected ? "text-primary" : "text-muted-foreground"
                        }`}
                      >
                        {device.name}
                      </p>
                      {showHostName && <p className="mt-0.5 text-[13px] font-medium text-muted-foreground">主机名 {hostNameLabel}</p>}
                    </div>
                  </div>
                  {isConnected ? (
                    <DisconnectIconButton
                      size="sm"
                      ariaLabel={disconnectTitle}
                      title={disconnectTitle}
                      onClick={() => onDisconnect(device)}
                    />
                  ) : (
                    <button
                      type="button"
                      aria-label={connectAction.ariaLabel}
                      disabled={connectAction.disabled}
                      title={connectAction.title}
                      onClick={() => onConnectDevice(device)}
                      className={`inline-flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-md text-muted-foreground ${SURFACE_REVEAL_BG} hover:bg-secondary hover:text-primary disabled:cursor-not-allowed disabled:opacity-40`}
                    >
                      <Zap size={13} aria-hidden="true" />
                    </button>
                  )}
                </div>
                <div className="space-y-1.5">
                  {[
                    { label: "地址", value: device.address, className: "text-secondary-foreground", mono: true },
                    { label: "系统", value: osLabel, className: "text-secondary-foreground", mono: false },
                    {
                      label: "最近活跃",
                      value: (() => {
                        const seen = device.lastPresenceAt ?? device.lastSeen;
                        return seen ? relativeTime(seen) : "—";
                      })(),
                      className: "text-secondary-foreground",
                      mono: false,
                    },
                  ].map((row) => (
                    <div key={row.label} className="flex items-center justify-between gap-3 leading-5">
                      <span className="text-[13px] font-medium text-muted-foreground">{row.label}</span>
                      <span className={`truncate text-[13px] font-medium ${row.mono ? "font-mono" : ""} ${row.className}`}>{row.value}</span>
                    </div>
                  ))}
                </div>
              </div>
            );
          })
        ) : (
          <div className="rounded-lg border border-dashed border-border px-3 py-6 text-center text-[13px] font-medium text-muted-foreground">
            暂无在线设备
          </div>
        )}
      </div>
    </ScrollArea>
  );
}
