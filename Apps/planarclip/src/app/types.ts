export type OS = "windows" | "macos";
export type NavId = "clipboard" | "devices" | "settings";
export type DeviceStatus = "connected" | "idle" | "offline";
export type ViewMode = "list" | "grid";
export type ColorScheme = "light" | "dark" | "system";
export type ClipType = "text" | "image" | "file";
export type AppConnectionStatus = "offline" | "connecting" | "online";
export type PairingStage =
  | "idle"
  | "manual_pairing"
  | "requesting_device"
  | "awaiting_code"
  | "submitting_code"
  | "incoming_request"
  | "incoming_pairing"
  | "incoming_accepting"
  | "error";

export type Device = {
  id: string;
  name: string;
  os: OS;
  host?: string;
  hostName?: string;
  port?: number;
  peerId?: string;
  address: string;
  status: DeviceStatus;
  lastSeen?: Date;
  pairedAt?: Date;
  latencyMs?: number;
  source: "discovery" | "connected" | "trusted";
  isTrusted?: boolean;
  autoAccept?: boolean;
  discoveredOnLan?: boolean;
  lastIp?: string | null;
};

export type DeviceBuckets = {
  paired: Device[];
  nearbyFamiliar: Device[];
  nearbyStranger: Device[];
  offline: Device[];
};

export type ClipEntry = {
  id: string;
  type: ClipType;
  content: string;
  sourceLabel: string;
  direction: "sent" | "received";
  size: string;
  timestamp: Date;
};

export type ThemeColor = {
  id: string;
  label: string;
  dark: { primary: string; accent: string; ring: string };
  light: { primary: string; accent: string; ring: string };
};

export type UiSettingsPayload = {
  color_scheme: ColorScheme;
  theme_color: string;
  device_name: string;
};

export type SettingAvailability = "editable" | "managed" | "planned";

export type ClipboardHistoryPayload = {
  id: string;
  content: string;
  source_label: string;
  direction: "sent" | "received";
  timestamp_ms: number;
};

export type LanDevicePayload = {
  name: string;
  peer_id: string;
  ip: string;
  host_name: string;
  port: number;
};

export type TrustedPeerPayload = {
  name: string;
  peer_id: string;
  last_ip?: string | null;
  auto_accept: boolean;
};

export type ConnectionRequestPayload = {
  device_name: string;
  peer_id: string;
  requires_pairing?: boolean;
};

export type ConnectionEstablishedPayload = {
  peer_name: string;
  peer_id: string;
  is_reconnect: boolean;
};

export type ConnectionFailedPayload = {
  kind?: string;
  message?: string;
};

export type ConnectionEndedPayload = {
  kind?: string;
  message?: string;
  peer_name?: string;
};

export type ConnectedPeer = {
  name: string;
  peerId?: string;
  address: string;
  os: OS;
  source: "lan" | "pair";
};

export type CommandExecutor = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
