import { Plus, Smartphone } from "lucide-react";
import type { AppConnectionStatus, Device } from "../../types";
import { OsIcon } from "../common/OsIcon";
import { StatusDot } from "../common/StatusDot";

type DevicesPageProps = {
  devices: Device[];
  connectionStatus: AppConnectionStatus;
  onShowPairing: () => void;
  onConnectDevice: (device: Device) => void;
  onDisconnect: () => void;
};

export function DevicesPage({ devices, connectionStatus, onShowPairing, onConnectDevice, onDisconnect }: DevicesPageProps) {
  const busyConnecting = connectionStatus === "connecting";
  const hasActiveSession = connectionStatus === "online";

  return (
    <div className="flex-1 overflow-y-auto px-4 pt-6 md:px-6 md:pt-8 xl:px-8">
      <div className="mb-6 flex max-w-3xl items-start justify-between gap-4">
        <div>
          <h2 className="mb-1 text-base font-semibold text-foreground">设备管理</h2>
          <p className="text-sm text-secondary-foreground">查看设备列表，并直接发起连接或断开当前会话。</p>
        </div>
        <button
          onClick={onShowPairing}
          disabled={busyConnecting}
          className="ml-4 flex shrink-0 items-center gap-2 rounded-lg bg-primary px-3.5 py-2 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
          type="button"
        >
          <Plus size={14} />
          {busyConnecting ? "正在连接…" : "连接新设备"}
        </button>
      </div>

      <div className="max-w-3xl space-y-2">
        <p className="mb-3 text-[13px] font-medium text-muted-foreground">设备列表</p>
        {devices.map((device) => {
          const connectDisabled = busyConnecting || hasActiveSession;
          const connectLabel = busyConnecting ? "处理中" : hasActiveSession ? "先断开再连接" : "连接设备";

          return (
            <div
              key={device.id}
              className="group flex items-center gap-4 rounded-xl border border-border bg-card p-4 transition-colors hover:border-primary/30"
            >
              <div className="rounded-lg bg-primary/10 p-2.5 text-primary">
                <OsIcon os={device.os} size={18} />
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium text-foreground">{device.name}</p>
                <p className="font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
              </div>
              <StatusDot status={device.status} size="lg" />
              {device.status === "connected" ? (
                <button
                  onClick={onDisconnect}
                  className="shrink-0 rounded-lg border border-border px-3 py-1.5 text-[13px] font-medium text-foreground transition-colors hover:border-destructive/30 hover:bg-destructive/10 hover:text-destructive"
                  type="button"
                >
                  断开连接
                </button>
              ) : (
                <button
                  onClick={() => onConnectDevice(device)}
                  disabled={connectDisabled}
                  className="shrink-0 rounded-lg bg-primary px-3 py-1.5 text-[13px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                  type="button"
                >
                  {connectLabel}
                </button>
              )}
            </div>
          );
        })}
        {devices.length === 0 && (
          <div className="py-12 text-center text-muted-foreground">
            <Smartphone size={32} className="mx-auto mb-3 opacity-30" />
            <p className="text-sm font-medium text-foreground">暂无可连接设备</p>
            <p className="mt-1 text-[13px] font-medium text-muted-foreground">保持双方在同一局域网，并确认对方应用已打开</p>
          </div>
        )}
      </div>
    </div>
  );
}
