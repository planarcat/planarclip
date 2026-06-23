import { Plus, PlugZap, RefreshCw, ShieldCheck, Smartphone, Unplug } from "lucide-react";
import type { ReactNode } from "react";
import type { AppConnectionStatus, Device } from "../../types";
import { relativeTime } from "../../utils/time";
import { OsIcon } from "../common/OsIcon";
import { StatusDot } from "../common/StatusDot";

type DevicesPageProps = {
  devices: Device[];
  connectionStatus: AppConnectionStatus;
  onShowPairing: () => void;
  onRefreshDevices: () => void;
  onConnectDevice: (device: Device) => void;
  onDisconnect: () => void;
  onRemoveTrustedPeer: (device: Device) => void;
  isRefreshingDevices: boolean;
};

type DeviceSectionHeaderProps = {
  accent: "cyan" | "amber";
  count: number;
  description?: string;
  icon: ReactNode;
  title: string;
  action?: ReactNode;
};

type KnownDeviceCardProps = {
  device: Device;
  connectDisabled: boolean;
  connectTitle: string;
  onConnectDevice: (device: Device) => void;
  onDisconnect: () => void;
  onRemoveTrustedPeer: (device: Device) => void;
};

type NearbyDeviceCardProps = {
  device: Device;
  disabled: boolean;
  title: string;
  onConnectDevice: (device: Device) => void;
};

function getOsLabel(device: Device) {
  return device.os === "macos" ? "macOS" : "Windows";
}

function getDeviceSubtitle(device: Device) {
  const hostNameLabel = device.hostName?.trim();
  return hostNameLabel && hostNameLabel.toLocaleLowerCase() !== device.name.trim().toLocaleLowerCase()
    ? `主机名 ${hostNameLabel}`
    : getOsLabel(device);
}

function DeviceSectionHeader({ accent, action, count, description, icon, title }: DeviceSectionHeaderProps) {
  const accentClassName = accent === "cyan" ? "text-primary" : "text-amber-400";

  return (
    <div className="flex items-center justify-between gap-3">
      <div className="flex min-w-0 items-center gap-2">
        <span className={accentClassName}>{icon}</span>
        <p className="text-[13px] font-medium text-primary">{title}</p>
        <span className="rounded-full bg-secondary px-2 py-0.5 text-[11px] font-medium text-muted-foreground">{count}</span>
      </div>
      <div className="flex shrink-0 items-center gap-3">
        {description && <p className="hidden text-[11px] font-medium text-muted-foreground sm:block">{description}</p>}
        {action}
      </div>
    </div>
  );
}

function EmptyDeviceSection({ message }: { message: string }) {
  return (
    <div className="rounded-xl border border-dashed border-border bg-card/60 px-4 py-10 text-center text-muted-foreground">
      <Smartphone size={30} className="mx-auto mb-3 opacity-30" />
      <p className="text-sm font-medium text-primary">{message}</p>
      <p className="mt-1 text-[13px] font-medium text-muted-foreground">保持双方在同一局域网，并确认对方应用已打开</p>
    </div>
  );
}

function KnownDeviceCard({
  device,
  connectDisabled,
  connectTitle,
  onConnectDevice,
  onDisconnect,
  onRemoveTrustedPeer,
}: KnownDeviceCardProps) {
  const osLabel = getOsLabel(device);
  const isConnected = device.status === "connected";
  const disconnectTitle = `断开与 ${device.name} 的连接`;
  const trustDescription = device.isTrusted
    ? device.lastIp
      ? `已信任，曾在 ${device.lastIp} 活跃`
      : "已信任，等待对方出现在同一局域网"
    : "当前会话已验证";

  return (
    <article className="overflow-hidden rounded-xl border border-border bg-card transition-colors hover:border-primary/30">
      <div className="flex items-center gap-4 px-4 py-3.5">
        <div className="rounded-lg bg-secondary p-2.5 text-muted-foreground">
          <OsIcon os={device.os} size={18} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold leading-none text-primary">{device.name}</p>
          <p className="mt-1.5 text-[13px] font-medium text-muted-foreground">{osLabel}</p>
          <p className="mt-0.5 truncate font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
        </div>
        <div className="flex shrink-0 items-center gap-3">
          <StatusDot status={device.status} size="md" />
          {isConnected ? (
            <button
              onClick={onDisconnect}
              aria-label={disconnectTitle}
              className="rounded-lg bg-secondary p-2 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
              title={disconnectTitle}
              type="button"
            >
              <Unplug size={15} />
            </button>
          ) : (
            <button
              onClick={() => onConnectDevice(device)}
              disabled={connectDisabled}
              aria-label={connectTitle}
              className="inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-2 text-[12px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
              title={connectTitle}
              type="button"
            >
              <PlugZap size={14} />
              建立连接
            </button>
          )}
        </div>
      </div>

      <div className="flex items-center justify-between gap-4 border-t border-border bg-secondary/20 px-4 py-2.5">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-[11px] font-medium text-muted-foreground">
          <span>{isConnected ? "当前会话已连接" : device.isTrusted ? "已配对设备" : "当前会话设备"}</span>
          <span>·</span>
          <span>最近活跃 {relativeTime(device.lastSeen)}</span>
        </div>
      </div>

      <div className="flex items-center justify-between gap-4 border-t border-border px-4 py-3">
        <div className="flex min-w-0 items-center gap-3">
          <ShieldCheck size={14} className="text-primary" />
          <div>
            <p className="text-[12px] font-medium text-primary">{device.isTrusted ? "信任该设备" : "已验证当前连接"}</p>
            <p className="mt-0.5 text-[11px] font-medium text-muted-foreground">{trustDescription}</p>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {isConnected && (
            <button
              onClick={onDisconnect}
              aria-label={disconnectTitle}
              className="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
              title={disconnectTitle}
              type="button"
            >
              <Unplug size={13} />
              断开连接
            </button>
          )}
          {device.isTrusted && (
            <button
              onClick={() => onRemoveTrustedPeer(device)}
              aria-label={`解除对 ${device.name} 的信任`}
              className="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
              title={`解除对 ${device.name} 的信任`}
              type="button"
            >
              解除信任
            </button>
          )}
        </div>
      </div>
    </article>
  );
}

function NearbyDeviceCard({ device, disabled, onConnectDevice, title }: NearbyDeviceCardProps) {
  return (
    <article className="flex items-center gap-4 rounded-xl border border-border bg-card px-4 py-3.5 transition-colors hover:border-primary/30">
      <div className="rounded-lg bg-secondary p-2.5 text-muted-foreground">
        <OsIcon os={device.os} size={18} />
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-semibold leading-none text-primary">{device.name}</p>
        <p className="mt-1.5 text-[13px] font-medium text-muted-foreground">{getDeviceSubtitle(device)}</p>
        <p className="mt-0.5 truncate font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
      </div>
      <button
        onClick={() => onConnectDevice(device)}
        disabled={disabled}
        aria-label={title}
        className="inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-primary px-3 py-2 text-[12px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
        title={title}
        type="button"
      >
        <PlugZap size={14} />
        建立连接
      </button>
    </article>
  );
}

export function DevicesPage({
  devices,
  connectionStatus,
  onShowPairing,
  onRefreshDevices,
  onConnectDevice,
  onDisconnect,
  onRemoveTrustedPeer,
  isRefreshingDevices,
}: DevicesPageProps) {
  const busyConnecting = connectionStatus === "connecting";
  const hasActiveSession = connectionStatus === "online";
  const knownDevices = devices.filter((device) => device.isTrusted || device.status === "connected" || device.source === "connected");
  const nearbyDevices = devices.filter((device) => !device.isTrusted && device.source === "discovery" && device.status !== "connected");

  return (
    <div className="flex-1 overflow-y-auto px-4 pt-6 md:px-6 md:pt-8 xl:px-8">
      <div className="mb-6 flex items-start justify-between gap-4">
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

      <div className="space-y-8">
        <section className="space-y-3">
          <DeviceSectionHeader
            accent="cyan"
            count={knownDevices.length}
            description="验证连接后自动归入此列表"
            icon={<ShieldCheck size={14} />}
            title="已配对设备"
          />
          {knownDevices.length > 0 ? (
            <div className="space-y-3">
              {knownDevices.map((device) => {
                const connectDisabled = busyConnecting || hasActiveSession || !device.host || !device.port;
                const connectTitle = !device.host || !device.port
                  ? "等待对方上线或刷新附近设备后再连接"
                  : busyConnecting
                    ? `正在处理 ${device.name} 的连接`
                    : hasActiveSession
                      ? "请先断开当前连接"
                      : `连接到 ${device.name}`;

                return (
                  <KnownDeviceCard
                    key={device.id}
                    device={device}
                    connectDisabled={connectDisabled}
                    connectTitle={connectTitle}
                    onConnectDevice={onConnectDevice}
                    onDisconnect={onDisconnect}
                    onRemoveTrustedPeer={onRemoveTrustedPeer}
                  />
                );
              })}
            </div>
          ) : (
            <EmptyDeviceSection message="暂无已配对设备" />
          )}
        </section>

        <section className="space-y-3">
          <DeviceSectionHeader
            accent="amber"
            count={nearbyDevices.length}
            icon={<Smartphone size={14} />}
            title="附近设备"
            action={
              <button
                onClick={onRefreshDevices}
                disabled={isRefreshingDevices}
                aria-label="刷新附近设备"
                className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground disabled:opacity-40"
                title="刷新附近设备"
                type="button"
              >
                <RefreshCw size={14} className={isRefreshingDevices ? "animate-spin" : undefined} />
              </button>
            }
          />
          {nearbyDevices.length > 0 ? (
            <div className="space-y-3">
              {nearbyDevices.map((device) => {
                const connectDisabled = busyConnecting || hasActiveSession;
                const actionTitle = busyConnecting
                  ? `正在处理 ${device.name} 的连接`
                  : hasActiveSession
                    ? "请先断开当前连接"
                    : `连接到 ${device.name}`;

                return (
                  <NearbyDeviceCard
                    key={device.id}
                    device={device}
                    disabled={connectDisabled}
                    title={actionTitle}
                    onConnectDevice={onConnectDevice}
                  />
                );
              })}
            </div>
          ) : (
            <EmptyDeviceSection message="暂无附近设备" />
          )}
        </section>
      </div>
    </div>
  );
}
