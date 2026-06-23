import { Plus, PlugZap, RefreshCw, Smartphone, Unplug } from "lucide-react";
import type { AppConnectionStatus, Device } from "../../types";
import { OsIcon } from "../common/OsIcon";
import { StatusDot } from "../common/StatusDot";

type DevicesPageProps = {
  devices: Device[];
  connectionStatus: AppConnectionStatus;
  onShowPairing: () => void;
  onRefreshDevices: () => void;
  onConnectDevice: (device: Device) => void;
  onDisconnect: () => void;
  isRefreshingDevices: boolean;
};

export function DevicesPage({
  devices,
  connectionStatus,
  onShowPairing,
  onRefreshDevices,
  onConnectDevice,
  onDisconnect,
  isRefreshingDevices,
}: DevicesPageProps) {
  const busyConnecting = connectionStatus === "connecting";
  const hasActiveSession = connectionStatus === "online";

  return (
    <div className="flex-1 overflow-y-auto px-4 pt-6 md:px-6 md:pt-8 xl:px-8">
      <div className="mb-6 flex max-w-3xl items-start justify-between gap-4">
        <div>
          <h2 className="mb-1 text-base font-semibold text-primary">设备管理</h2>
          <p className="text-sm text-secondary-foreground">查看设备列表，并直接发起连接或断开当前会话。</p>
        </div>
        <button
          onClick={onShowPairing}
          disabled={busyConnecting}
          aria-label={busyConnecting ? "正在连接新设备" : "连接新设备"}
          className="ml-4 shrink-0 rounded-lg bg-primary p-2 text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
          title={busyConnecting ? "正在连接新设备" : "连接新设备"}
          type="button"
        >
          <Plus size={16} />
        </button>
      </div>

      <div className="max-w-3xl space-y-2">
        <div className="mb-3 flex items-center justify-between gap-2">
          <p className="text-[13px] font-medium text-primary">设备列表</p>
          <button
            onClick={onRefreshDevices}
            disabled={isRefreshingDevices}
            aria-label="刷新设备列表"
            className="rounded-md p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-foreground disabled:opacity-40"
            title="刷新设备列表"
            type="button"
          >
            <RefreshCw size={14} className={isRefreshingDevices ? "animate-spin" : undefined} />
          </button>
        </div>
        {devices.map((device) => {
          const connectDisabled = busyConnecting || hasActiveSession;
          const actionTitle =
            device.status === "connected"
              ? `断开与 ${device.name} 的连接`
              : busyConnecting
                ? `正在处理 ${device.name} 的连接`
                : hasActiveSession
                  ? "请先断开当前连接"
                  : `连接到 ${device.name}`;

          return (
            <div
              key={device.id}
              className="group flex items-center gap-4 rounded-xl border border-border bg-card p-4 transition-colors hover:border-primary/30"
            >
              <div className="rounded-lg bg-primary/10 p-2.5 text-primary">
                <OsIcon os={device.os} size={18} />
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium text-primary">{device.name}</p>
                <p className="mt-0.5 text-[13px] font-medium text-muted-foreground">
                  {device.hostName ? `主机名 ${device.hostName}` : device.os === "macos" ? "macOS 系统" : "Windows 系统"}
                </p>
                <p className="font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                <StatusDot status={device.status} size="md" />
                {device.status === "connected" ? (
                  <button
                    onClick={onDisconnect}
                    aria-label={actionTitle}
                    className="rounded-lg border border-border p-2 text-foreground transition-colors hover:border-destructive/30 hover:bg-destructive/10 hover:text-destructive"
                    title={actionTitle}
                    type="button"
                  >
                    <Unplug size={15} />
                  </button>
                ) : (
                  <button
                    onClick={() => onConnectDevice(device)}
                    disabled={connectDisabled}
                    aria-label={actionTitle}
                    className="rounded-lg bg-primary p-2 text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                    title={actionTitle}
                    type="button"
                  >
                    <PlugZap size={15} />
                  </button>
                )}
              </div>
            </div>
          );
        })}
        {devices.length === 0 && (
          <div className="py-12 text-center text-muted-foreground">
            <Smartphone size={32} className="mx-auto mb-3 opacity-30" />
            <p className="text-sm font-medium text-primary">暂无可连接设备</p>
            <p className="mt-1 text-[13px] font-medium text-muted-foreground">保持双方在同一局域网，并确认对方应用已打开</p>
          </div>
        )}
      </div>
    </div>
  );
}
