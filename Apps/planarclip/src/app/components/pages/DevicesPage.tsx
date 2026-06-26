import {
  CloudOff,
  Plus,
  RefreshCw,
  Shield,
  ShieldCheck,
  ShieldOff,
  Smartphone,
  Unplug,
  UserX,
  Zap,
} from "lucide-react";
import type { ReactNode } from "react";
import type { AppConnectionStatus, Device } from "../../types";
import { MAX_CONNECTIONS } from "../../constants/connection";
import { categorizeDevices } from "../../utils/device";
import { relativeTime } from "../../utils/time";
import { OsIcon } from "../common/OsIcon";

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

function getTrustTooltip(autoAccept: boolean) {
  if (autoAccept) {
    return "已信任该设备：对方发起连接时将自动接受。点击后，下次连接需要你手动确认。";
  }

  return "未信任该设备：对方发起连接时需你手动确认。点击后，对方发起连接时将自动接受。";
}

function DeviceSectionHeader({ accent, action, count, icon, title }: DeviceSectionHeaderProps) {
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

function HoverTooltip({ children, content }: { children: ReactNode; content: string }) {
  return (
    <span className="group/tooltip relative inline-flex">
      {children}
      <span
        role="tooltip"
        className="pointer-events-none absolute left-[calc(100%+10px)] top-1/2 z-20 w-max max-w-[240px] -translate-y-1/2 rounded-lg border border-border bg-card px-2.5 py-1.5 text-[11px] font-medium leading-snug text-muted-foreground opacity-0 shadow-md transition-opacity group-hover/tooltip:opacity-100 group-focus-within/tooltip:opacity-100"
      >
        <span
          aria-hidden="true"
          className="absolute right-full top-1/2 -translate-y-1/2 border-[6px] border-transparent border-r-border"
        />
        <span
          aria-hidden="true"
          className="absolute right-full top-1/2 mr-px -translate-y-1/2 border-[5px] border-transparent border-r-card"
        />
        {content}
      </span>
    </span>
  );
}

function TrustShieldButton({
  device,
  onSetPeerAutoAccept,
}: {
  device: Device;
  onSetPeerAutoAccept: (device: Device, autoAccept: boolean) => void;
}) {
  const autoAccept = Boolean(device.autoAccept);
  const label = autoAccept ? `不再信任 ${device.name}` : `信任 ${device.name} 的来访`;

  return (
    <HoverTooltip content={getTrustTooltip(autoAccept)}>
      <button
        type="button"
        aria-label={label}
        aria-pressed={autoAccept}
        onClick={() => onSetPeerAutoAccept(device, !autoAccept)}
        className="inline-flex shrink-0 items-center justify-center rounded-md p-0.5 transition-colors hover:bg-secondary/60"
      >
        {autoAccept ? (
          <Shield size={13} className="text-primary" aria-hidden="true" />
        ) : (
          <ShieldOff size={13} className="text-primary/40" aria-hidden="true" />
        )}
      </button>
    </HoverTooltip>
  );
}

function DeviceNameRow({
  device,
  onSetPeerAutoAccept,
}: {
  device: Device;
  onSetPeerAutoAccept: (device: Device, autoAccept: boolean) => void;
}) {
  return (
    <div className="flex min-w-0 items-center gap-1.5">
      <p className="truncate text-sm font-semibold leading-none text-primary">{device.name}</p>
      {device.isTrusted ? (
        <TrustShieldButton device={device} onSetPeerAutoAccept={onSetPeerAutoAccept} />
      ) : (
        <HoverTooltip content="该设备尚未加入熟悉列表，重新配对成功后才能设置自动接受连接。">
          <span
            className="inline-flex shrink-0 cursor-not-allowed items-center justify-center rounded-md p-0.5"
            aria-label="尚未加入熟悉列表，暂不可设置自动接受连接"
          >
            <ShieldOff size={13} className="text-primary/25" aria-hidden="true" />
          </span>
        </HoverTooltip>
      )}
    </div>
  );
}

function DeviceIconButton({
  ariaLabel,
  disabled,
  hoverDestructive,
  icon: Icon,
  onClick,
  title,
}: {
  ariaLabel: string;
  disabled?: boolean;
  hoverDestructive?: boolean;
  icon: typeof Zap;
  onClick: () => void;
  title: string;
}) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={onClick}
      title={title}
      className={`inline-flex h-[26px] w-[26px] shrink-0 items-center justify-center rounded-[5.5px] text-muted-foreground transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${
        hoverDestructive ? "hover:text-destructive" : "hover:text-foreground"
      }`}
    >
      <Icon size={14} aria-hidden="true" />
    </button>
  );
}

function RemoveDeviceIconButton({
  device,
  onRemoveTrustedPeer,
}: {
  device: Device;
  onRemoveTrustedPeer: (device: Device) => void;
}) {
  const title = `移除 ${device.name}`;

  return (
    <DeviceIconButton
      ariaLabel={title}
      hoverDestructive
      icon={UserX}
      onClick={() => onRemoveTrustedPeer(device)}
      title="移除后，该设备将变为陌生设备"
    />
  );
}

function KnownDeviceCard({
  device,
  onDisconnect,
  onRemoveTrustedPeer,
  onSetPeerAutoAccept,
}: KnownDeviceCardProps) {
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
          <DeviceNameRow device={device} onSetPeerAutoAccept={onSetPeerAutoAccept} />
          <p className="mt-1.5 text-[13px] font-medium text-muted-foreground">{osLabel}</p>
          <p className="mt-0.5 truncate font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {device.isTrusted && <RemoveDeviceIconButton device={device} onRemoveTrustedPeer={onRemoveTrustedPeer} />}
          <DeviceIconButton
            ariaLabel={disconnectTitle}
            hoverDestructive
            icon={Unplug}
            onClick={onDisconnect}
            title={disconnectTitle}
          />
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
          <DeviceNameRow device={device} onSetPeerAutoAccept={onSetPeerAutoAccept} />
          <p className="mt-1.5 text-[13px] font-medium text-muted-foreground">{getOsLabel(device)}</p>
          <p className="mt-0.5 truncate font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {device.isTrusted && <RemoveDeviceIconButton device={device} onRemoveTrustedPeer={onRemoveTrustedPeer} />}
          <DeviceIconButton
            ariaLabel={connectTitle}
            disabled={connectDisabled}
            icon={Zap}
            onClick={() => onConnectDevice(device)}
            title={connectTitle}
          />
        </div>
      </div>
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
        <DeviceIconButton
          ariaLabel={connectTitle}
          disabled={connectDisabled}
          icon={Zap}
          onClick={() => onConnectDevice(device)}
          title={connectTitle}
        />
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
          <DeviceNameRow device={device} onSetPeerAutoAccept={onSetPeerAutoAccept} />
          <p className="mt-1.5 text-[13px] font-medium text-muted-foreground">{getOsLabel(device)}</p>
          <p className="mt-0.5 truncate font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
        </div>
        <RemoveDeviceIconButton device={device} onRemoveTrustedPeer={onRemoveTrustedPeer} />
      </div>

      {lastOnlineLabel && (
        <div className="border-t border-border bg-secondary/20 px-4 py-2">
          <p className="text-[11px] font-medium text-muted-foreground">{lastOnlineLabel}</p>
        </div>
      )}
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
  const connectedCount = devices.filter((device) => device.status === "connected").length;
  const atConnectionLimit = connectedCount >= MAX_CONNECTIONS;
  const { paired, nearbyFamiliar, nearbyStranger, offline } = categorizeDevices(devices);
  const nearbyDevices = [...nearbyFamiliar, ...nearbyStranger];

  const buildConnectState = (device: Device) => {
    const connectDisabled = busyConnecting || (atConnectionLimit && device.status !== "connected") || !device.host || !device.port;
    const connectTitle = !device.host || !device.port
      ? "等待对方上线或刷新附近设备后再连接"
      : busyConnecting
        ? `正在处理 ${device.name} 的连接`
        : atConnectionLimit
          ? "已超出连接上限，请先断开其中一个设备"
          : device.status === "connected"
            ? `重新连接到 ${device.name}`
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
          onClick={() => onShowPairing()}
          disabled={busyConnecting || atConnectionLimit}
          aria-label={busyConnecting ? "正在连接新设备" : atConnectionLimit ? "已超出连接上限" : "连接新设备"}
          className="ml-4 shrink-0 rounded-lg bg-primary p-2 text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
          title={busyConnecting ? "正在连接新设备" : atConnectionLimit ? "已超出连接上限" : "连接新设备"}
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
            <EmptyDeviceSection message="暂无已连接设备" />
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

