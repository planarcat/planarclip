import {
  CloudOff,
  HelpCircle,
  Plus,
  PlugZap,
  History,
  RefreshCw,
  ShieldCheck,
  Smartphone,
  Unplug,
  UserMinus,
} from "lucide-react";
import type { ReactNode } from "react";
import type { AppConnectionStatus, Device } from "../../types";
import { categorizeDevices } from "../../utils/device";
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
  onSetPeerAutoAccept: (device: Device, autoAccept: boolean) => void;
  isRefreshingDevices: boolean;
};

type DeviceSectionHeaderProps = {
  accent: "emerald" | "cyan" | "muted";
  count: number;
  description?: string;
  icon: ReactNode;
  title: string;
  action?: ReactNode;
};

type ConnectableDeviceCardProps = {
  device: Device;
  connectDisabled: boolean;
  connectTitle: string;
  onConnectDevice: (device: Device) => void;
  onRemoveTrustedPeer: (device: Device) => void;
};

type KnownDeviceCardProps = {
  device: Device;
  onDisconnect: () => void;
  onRemoveTrustedPeer: (device: Device) => void;
  onSetPeerAutoAccept: (device: Device, autoAccept: boolean) => void;
};

type OfflineDeviceCardProps = {
  device: Device;
  onRemoveTrustedPeer: (device: Device) => void;
  onSetPeerAutoAccept: (device: Device, autoAccept: boolean) => void;
};

type FamiliarDeviceCardProps = ConnectableDeviceCardProps & {
  onSetPeerAutoAccept: (device: Device, autoAccept: boolean) => void;
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

function formatActivityMeta(device: Device) {
  const segments: string[] = [];

  if (device.pairedAt) {
    segments.push(`配对于 ${relativeTime(device.pairedAt)}`);
  }

  if (device.lastSeen) {
    segments.push(`最近活跃 ${relativeTime(device.lastSeen)}`);
  }

  return segments.join(" · ");
}

function DeviceSectionHeader({ accent, action, count, description, icon, title }: DeviceSectionHeaderProps) {
  const accentClassName =
    accent === "emerald" ? "text-emerald-400" : accent === "cyan" ? "text-primary" : "text-muted-foreground";

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

function TrustToggle({
  checked,
  disabled,
  onChange,
  ariaLabel,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
  ariaLabel: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`inline-flex h-[19px] w-[34px] shrink-0 items-center rounded-full p-[2px] transition-colors disabled:cursor-not-allowed disabled:opacity-60 ${
        checked ? "bg-primary" : "bg-[var(--switch-background)]"
      }`}
    >
      <span
        aria-hidden="true"
        className={`block h-[15px] w-[15px] rounded-full bg-white transition-transform duration-200 ${
          checked ? "translate-x-[15px]" : "translate-x-0"
        }`}
      />
    </button>
  );
}

function RemoveDeviceButton({ device, onRemoveTrustedPeer }: { device: Device; onRemoveTrustedPeer: (device: Device) => void }) {
  const title = `移除 ${device.name}`;

  return (
    <button
      onClick={() => onRemoveTrustedPeer(device)}
      aria-label={title}
      className="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
      title="移除后，该设备将变为陌生设备"
      type="button"
    >
      <UserMinus size={12} />
      移除
    </button>
  );
}

function DeviceTrustRow({
  device,
  disabled,
  onRemoveTrustedPeer,
  onSetPeerAutoAccept,
}: {
  device: Device;
  disabled?: boolean;
  onRemoveTrustedPeer: (device: Device) => void;
  onSetPeerAutoAccept: (device: Device, autoAccept: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4 border-t border-border px-4 py-3">
      <div className="flex min-w-0 items-center gap-2">
        <ShieldCheck size={14} className="text-muted-foreground" />
        <p className="text-[12px] font-medium text-muted-foreground">自动接受连接</p>
        <span
          className="inline-flex h-[13px] w-[13px] items-center justify-center rounded-full border border-muted-foreground/50 text-[9px] text-muted-foreground"
          title="开启后，该熟悉设备发起连接时会直接建立会话；关闭后仍保留为熟悉设备，但需要你确认。"
        >
          <HelpCircle size={9} aria-hidden="true" />
        </span>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <TrustToggle
          checked={Boolean(device.autoAccept)}
          disabled={disabled}
          ariaLabel={`${device.name} 的自动接受连接`}
          onChange={(checked) => onSetPeerAutoAccept(device, checked)}
        />
        <RemoveDeviceButton device={device} onRemoveTrustedPeer={onRemoveTrustedPeer} />
      </div>
    </div>
  );
}

function KnownDeviceCard({ device, onDisconnect, onRemoveTrustedPeer, onSetPeerAutoAccept }: KnownDeviceCardProps) {
  const osLabel = getOsLabel(device);
  const disconnectTitle = `断开与 ${device.name} 的连接`;
  const activityMeta = formatActivityMeta(device);

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
          <button
            onClick={onDisconnect}
            aria-label={disconnectTitle}
            className="rounded-lg bg-secondary p-2 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
            title={disconnectTitle}
            type="button"
          >
            <Unplug size={15} />
          </button>
        </div>
      </div>

      {(activityMeta || device.latencyMs != null) && (
        <div className="flex items-center justify-between gap-4 border-t border-border bg-secondary/20 px-4 py-2.5">
          {activityMeta ? (
            <p className="min-w-0 text-[11px] font-medium text-muted-foreground">{activityMeta}</p>
          ) : (
            <span />
          )}
          {device.latencyMs != null && (
            <p className="shrink-0 font-mono text-[11px] font-medium text-emerald-400">{device.latencyMs}ms</p>
          )}
        </div>
      )}

      {device.isTrusted && (
        <DeviceTrustRow
          device={device}
          disabled={device.status === "connected"}
          onRemoveTrustedPeer={onRemoveTrustedPeer}
          onSetPeerAutoAccept={onSetPeerAutoAccept}
        />
      )}
    </article>
  );
}

function NearbyFamiliarCard({
  device,
  connectDisabled,
  connectTitle,
  onConnectDevice,
  onRemoveTrustedPeer,
  onSetPeerAutoAccept,
}: FamiliarDeviceCardProps) {
  return (
    <article className="overflow-hidden rounded-xl border border-border bg-card transition-colors hover:border-primary/30">
      <div className="flex items-center gap-4 px-4 py-3.5">
        <div className="rounded-lg bg-secondary p-2.5 text-muted-foreground">
          <OsIcon os={device.os} size={18} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-1.5">
            <p className="truncate text-sm font-semibold leading-none text-emerald-400">{device.name}</p>
            <span title="曾连接过的熟悉设备">
              <History size={13} className="shrink-0 text-emerald-400" aria-label="曾连接过的熟悉设备" />
            </span>
          </div>
          <p className="mt-1.5 text-[13px] font-medium text-muted-foreground">{getOsLabel(device)}</p>
          <p className="mt-0.5 truncate font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
        </div>
        <button
          onClick={() => onConnectDevice(device)}
          disabled={connectDisabled}
          aria-label={connectTitle}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-primary px-3 py-2 text-[12px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
          title={connectTitle}
          type="button"
        >
          <PlugZap size={14} />
          重新连接
        </button>
      </div>

      <DeviceTrustRow
        device={device}
        onRemoveTrustedPeer={onRemoveTrustedPeer}
        onSetPeerAutoAccept={onSetPeerAutoAccept}
      />
    </article>
  );
}

function NearbyStrangerCard({ device, connectDisabled, connectTitle, onConnectDevice }: Omit<ConnectableDeviceCardProps, "onRemoveTrustedPeer">) {
  return (
    <article className="overflow-hidden rounded-xl border border-border bg-card transition-colors hover:border-primary/30">
      <div className="flex items-center gap-4 px-4 py-3.5">
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
          disabled={connectDisabled}
          aria-label={connectTitle}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-border bg-secondary px-3 py-2 text-[12px] font-medium text-primary transition-colors hover:border-primary/30 hover:bg-secondary/80 disabled:opacity-40"
          title={connectTitle}
          type="button"
        >
          <PlugZap size={14} />
          建立连接
        </button>
      </div>
    </article>
  );
}

function OfflineDeviceCard({ device, onRemoveTrustedPeer, onSetPeerAutoAccept }: OfflineDeviceCardProps) {
  const lastOnlineLabel = device.lastSeen ? `最近在线 ${relativeTime(device.lastSeen)}` : "";

  return (
    <article className="overflow-hidden rounded-xl border border-border bg-card opacity-70 transition-colors hover:border-primary/30">
      <div className="flex items-center gap-4 px-4 py-3.5">
        <div className="rounded-lg bg-secondary p-2.5 text-muted-foreground">
          <OsIcon os={device.os} size={18} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold leading-none text-primary">{device.name}</p>
          <p className="mt-1.5 text-[13px] font-medium text-muted-foreground">{getOsLabel(device)}</p>
          <p className="mt-0.5 truncate font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <CloudOff size={14} className="text-primary/60" aria-hidden="true" />
          <RemoveDeviceButton device={device} onRemoveTrustedPeer={onRemoveTrustedPeer} />
        </div>
      </div>

      <div className="flex items-center justify-between gap-4 border-t border-border bg-secondary/20 px-4 py-2">
        <p className="text-[11px] font-medium text-muted-foreground">{lastOnlineLabel}</p>
        <div className="flex shrink-0 items-center gap-2">
          <TrustToggle
            checked={Boolean(device.autoAccept)}
            ariaLabel={`${device.name} 的自动接受连接`}
            onChange={(checked) => onSetPeerAutoAccept(device, checked)}
          />
        </div>
      </div>
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
  onSetPeerAutoAccept,
  isRefreshingDevices,
}: DevicesPageProps) {
  const busyConnecting = connectionStatus === "connecting";
  const hasActiveSession = connectionStatus === "online";
  const { paired, nearbyFamiliar, nearbyStranger, offline } = categorizeDevices(devices);
  const nearbyDevices = [...nearbyFamiliar, ...nearbyStranger];

  const buildConnectState = (device: Device) => {
    const connectDisabled = busyConnecting || hasActiveSession || !device.host || !device.port;
    const connectTitle = !device.host || !device.port
      ? "等待对方上线或刷新附近设备后再连接"
      : busyConnecting
        ? `正在处理 ${device.name} 的连接`
        : hasActiveSession
          ? "请先断开当前连接"
          : `连接到 ${device.name}`;

    return { connectDisabled, connectTitle };
  };

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
            accent="emerald"
            count={paired.length}
            description={paired.length > 0 ? "在线且已建立连接" : undefined}
            icon={<ShieldCheck size={14} />}
            title="已配对"
          />
          {paired.length > 0 ? (
            <div className="space-y-3">
              {paired.map((device) => (
                <KnownDeviceCard
                  key={device.id}
                  device={device}
                  onDisconnect={onDisconnect}
                  onRemoveTrustedPeer={onRemoveTrustedPeer}
                  onSetPeerAutoAccept={onSetPeerAutoAccept}
                />
              ))}
            </div>
          ) : (
            <EmptyDeviceSection message="暂无在线设备" />
          )}
        </section>

        <section className="space-y-3">
          <DeviceSectionHeader
            accent="cyan"
            count={nearbyDevices.length}
            icon={<Smartphone size={14} />}
            title="附近"
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
              {nearbyFamiliar.map((device) => {
                const { connectDisabled, connectTitle } = buildConnectState(device);

                return (
                  <NearbyFamiliarCard
                    key={device.id}
                    device={device}
                    connectDisabled={connectDisabled}
                    connectTitle={connectTitle}
                    onConnectDevice={onConnectDevice}
                    onRemoveTrustedPeer={onRemoveTrustedPeer}
                    onSetPeerAutoAccept={onSetPeerAutoAccept}
                  />
                );
              })}
              {nearbyStranger.map((device) => {
                const { connectDisabled, connectTitle } = buildConnectState(device);

                return (
                  <NearbyStrangerCard
                    key={device.id}
                    device={device}
                    connectDisabled={connectDisabled}
                    connectTitle={connectTitle}
                    onConnectDevice={onConnectDevice}
                  />
                );
              })}
            </div>
          ) : (
            <EmptyDeviceSection message="暂无附近设备" />
          )}
        </section>

        <section className="space-y-3">
          <DeviceSectionHeader
            accent="muted"
            count={offline.length}
            icon={<CloudOff size={14} />}
            title="离线"
          />
          {offline.length > 0 ? (
            <div className="space-y-3">
              {offline.map((device) => (
                <OfflineDeviceCard
                  key={device.id}
                  device={device}
                  onRemoveTrustedPeer={onRemoveTrustedPeer}
                  onSetPeerAutoAccept={onSetPeerAutoAccept}
                />
              ))}
            </div>
          ) : (
            <EmptyDeviceSection message="暂无离线设备" />
          )}
        </section>
      </div>
    </div>
  );
}
