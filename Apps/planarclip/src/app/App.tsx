import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Apple,
  Clipboard,
  Copy,
  File,
  FileText,
  Image,
  LayoutGrid,
  LayoutList,
  Loader2,
  Monitor,
  Moon,
  Palette,
  Plus,
  Radio,
  Settings,
  Shield,
  ShieldCheck,
  Smartphone,
  Sun,
  SunMoon,
  Wifi,
  WifiOff,
  X,
  Zap,
} from "lucide-react";

type OS = "windows" | "macos";
type NavId = "clipboard" | "devices" | "settings";
type DeviceStatus = "connected" | "idle" | "offline";
type ViewMode = "list" | "grid";
type ColorScheme = "light" | "dark" | "system";
type ClipType = "text" | "image" | "file";
type AppConnectionStatus = "offline" | "connecting" | "online";
type PairingStage =
  | "idle"
  | "manual_pairing"
  | "requesting_device"
  | "awaiting_code"
  | "submitting_code"
  | "incoming_request"
  | "error";

type Device = {
  id: string;
  name: string;
  os: OS;
  host?: string;
  port?: number;
  peerId?: string;
  address: string;
  status: DeviceStatus;
  lastSeen?: Date;
  source: "discovery" | "connected";
};

type ClipEntry = {
  id: string;
  type: ClipType;
  content: string;
  sourceLabel: string;
  direction: "sent" | "received";
  size: string;
  timestamp: Date;
};

type ThemeColor = {
  id: string;
  label: string;
  dark: { primary: string; accent: string; ring: string };
  light: { primary: string; accent: string; ring: string };
};

type UiSettingsPayload = {
  color_scheme: ColorScheme;
  theme_color: string;
};

type SettingAvailability = "editable" | "managed" | "planned";

type ClipboardHistoryPayload = {
  id: string;
  content: string;
  source_label: string;
  direction: "sent" | "received";
  timestamp_ms: number;
};

type LanDevicePayload = {
  name: string;
  peer_id: string;
  ip: string;
  port: number;
};

type ConnectionRequestPayload = {
  device_name: string;
  peer_id: string;
  pairing_code: string;
};

type ConnectionEstablishedPayload = {
  peer_name: string;
  peer_id: string;
  is_reconnect: boolean;
};

type ConnectionFailedPayload = {
  kind?: string;
  message?: string;
};

type ConnectionEndedPayload = {
  kind?: string;
  message?: string;
  peer_name?: string;
};

type ConnectedPeer = {
  name: string;
  peerId?: string;
  address: string;
  os: OS;
  source: "lan" | "pair";
};

const TAURI_AVAILABLE = isTauri();
const EMPTY_CLIPS: ClipEntry[] = [];
const PREVIEW_UI_SETTINGS_KEY = "planarclip-ui-settings";

const THEME_COLORS: ThemeColor[] = [
  {
    id: "cyan",
    label: "青色",
    dark: { primary: "#22d3ee", accent: "#0e7490", ring: "#22d3ee" },
    light: { primary: "#0891b2", accent: "#0c4a6e", ring: "#0891b2" },
  },
  {
    id: "violet",
    label: "紫色",
    dark: { primary: "#a78bfa", accent: "#7c3aed", ring: "#a78bfa" },
    light: { primary: "#7c3aed", accent: "#5b21b6", ring: "#7c3aed" },
  },
  {
    id: "emerald",
    label: "绿色",
    dark: { primary: "#34d399", accent: "#059669", ring: "#34d399" },
    light: { primary: "#059669", accent: "#065f46", ring: "#059669" },
  },
  {
    id: "rose",
    label: "玫红",
    dark: { primary: "#fb7185", accent: "#e11d48", ring: "#fb7185" },
    light: { primary: "#e11d48", accent: "#9f1239", ring: "#e11d48" },
  },
];

const DEFAULT_UI_SETTINGS: UiSettingsPayload = {
  color_scheme: "dark",
  theme_color: THEME_COLORS[0].id,
};

function normalizeColorScheme(value?: string): ColorScheme {
  if (value === "light" || value === "dark" || value === "system") {
    return value;
  }

  return DEFAULT_UI_SETTINGS.color_scheme;
}

function getThemeById(themeId?: string) {
  return THEME_COLORS.find((theme) => theme.id === themeId) ?? THEME_COLORS[0];
}

function loadPreviewUiSettings(): UiSettingsPayload {
  if (typeof window === "undefined") {
    return DEFAULT_UI_SETTINGS;
  }

  try {
    const raw = window.localStorage.getItem(PREVIEW_UI_SETTINGS_KEY);
    if (!raw) {
      return DEFAULT_UI_SETTINGS;
    }

    const parsed = JSON.parse(raw) as Partial<UiSettingsPayload>;
    return {
      color_scheme: normalizeColorScheme(parsed.color_scheme),
      theme_color: getThemeById(parsed.theme_color).id,
    };
  } catch {
    return DEFAULT_UI_SETTINGS;
  }
}

function savePreviewUiSettings(settings: UiSettingsPayload) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(PREVIEW_UI_SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // 忽略预览态下的本地存储失败，保持界面继续可用。
  }
}

function formatClipSize(content: string) {
  const bytes = new TextEncoder().encode(content).length;
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function mapClipboardHistory(payload: ClipboardHistoryPayload[]): ClipEntry[] {
  return payload.map((item) => ({
    id: item.id,
    type: "text",
    content: item.content,
    sourceLabel: item.source_label,
    direction: item.direction,
    size: formatClipSize(item.content),
    timestamp: new Date(item.timestamp_ms),
  }));
}

function relativeTime(date?: Date) {
  if (!date) return "刚刚";

  const diff = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (diff < 60) return `${diff}秒前`;
  if (diff < 3_600) return `${Math.floor(diff / 60)}分钟前`;
  if (diff < 86_400) return `${Math.floor(diff / 3_600)}小时前`;
  return `${Math.floor(diff / 86_400)}天前`;
}

function formatTime() {
  return new Date().toLocaleTimeString();
}

function isDarkActive() {
  return document.documentElement.classList.contains("dark");
}

function applyColorScheme(scheme: ColorScheme) {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.classList.toggle(
    "dark",
    scheme === "dark" || (scheme === "system" && prefersDark),
  );
}

function applyThemeColor(theme: ThemeColor) {
  const selected = isDarkActive() ? theme.dark : theme.light;
  const root = document.documentElement;
  root.style.setProperty("--primary", selected.primary);
  root.style.setProperty("--accent", selected.accent);
  root.style.setProperty("--ring", selected.ring);
  root.style.setProperty("--primary-foreground", isDarkActive() ? "#030b10" : "#ffffff");
}

function useRelativeTicker() {
  const [, setTick] = useState(0);

  useEffect(() => {
    const timer = window.setInterval(() => setTick((value) => value + 1), 5_000);
    return () => window.clearInterval(timer);
  }, []);
}

function inferOs(name: string): OS {
  return /mac|iphone|ipad|ios/i.test(name) ? "macos" : "windows";
}

function rawMessage(error: unknown) {
  if (error && typeof error === "object" && "message" in error) {
    const { message } = error as { message?: unknown };
    if (typeof message === "string" && message.trim()) {
      return message.trim();
    }
  }

  return String(error ?? "").trim();
}

/**
 * 把 Tauri 和 Rust 返回的技术错误折叠成稳定的中文提示，避免界面直接暴露底层实现细节。
 */
function normalizeUserMessage(error: unknown, fallback: string, targetName?: string) {
  const raw = rawMessage(error);

  if (!raw) {
    return fallback;
  }

  if (raw.includes("配对码必须为 6 位数字")) {
    return "请输入 6 位数字配对码。";
  }

  if (raw.includes("当前没有待处理的连接")) {
    return "这次连接已经结束，请重新选择设备后再试。";
  }

  if (raw.includes("密钥对尚未初始化")) {
    return "设备还在准备连接信息，请稍后再试。";
  }

  if (raw.includes("对方已拒绝连接") || raw.includes("对方已拒绝这次连接")) {
    return "对方没有继续这次连接，请重新发起连接。";
  }

  if (raw.includes("配对码已过期") || raw.includes("这次连接已超时")) {
    return "这次配对已超时，请重新发起连接并输入新的配对码。";
  }

  if (raw.includes("配对码无效") || raw.includes("配对码不正确")) {
    return "配对码不正确，或这次连接已经失效，请重新核对后再试。";
  }

  if (raw.includes("已取消") || raw.includes("用户已取消")) {
    return "这次连接已经取消，你可以重新选择设备。";
  }

  if (raw.includes("已断开连接")) {
    return raw;
  }

  if (
    raw.includes("I/O 错误") ||
    raw.includes("I/O error") ||
    raw.includes("os error 10061") ||
    raw.includes("无法连接") ||
    raw.includes("actively refused") ||
    raw.includes("暂时无法连接对方设备")
  ) {
    if (targetName) {
      return `暂时连不上 ${targetName}，请确认对方应用已打开，而且你们在同一局域网内。`;
    }
    return "暂时无法连接对方设备，请确认对方应用已打开，而且你们在同一局域网内。";
  }

  if (raw.includes("协议错误") || raw.includes("帧错误") || raw.includes("连接过程中出了点问题")) {
    return "连接过程中出了点问题，请重新发起连接。";
  }

  if (raw.startsWith("连接失败：") || raw.startsWith("配对失败：")) {
    return normalizeUserMessage(raw.replace(/^[^：]+：/, "").trim(), fallback, targetName);
  }

  if (raw.includes("浏览器预览模式") || raw.includes("桌面端")) {
    return "当前是浏览器预览模式，请在桌面应用中体验连接能力。";
  }

  return fallback;
}

async function callCommand<T>(command: string, args?: Record<string, unknown>) {
  if (!TAURI_AVAILABLE) {
    throw new Error("当前是浏览器预览模式，请在桌面应用中体验连接能力。");
  }

  return invoke<T>(command, args);
}

function createDeviceId(prefix: string, value: string) {
  return `${prefix}:${value}`;
}

/**
 * 设备列表优先反映真实的局域网发现结果；如果当前连接对象不在发现列表里，则补一条合成设备卡，避免界面丢失当前会话。
 */
function buildDevices(lanDevices: LanDevicePayload[], connectedPeer: ConnectedPeer | null) {
  const deviceMap = new Map<string, Device>();

  lanDevices.forEach((device) => {
    const isConnected =
      connectedPeer != null &&
      (connectedPeer.peerId === device.peer_id || connectedPeer.name === device.name);

    deviceMap.set(device.peer_id, {
      id: createDeviceId("lan", device.peer_id),
      name: device.name,
      os: inferOs(device.name),
      host: device.ip,
      port: device.port,
      peerId: device.peer_id,
      address: `${device.ip}:${device.port}`,
      status: isConnected ? "connected" : "idle",
      lastSeen: new Date(),
      source: "discovery",
    });
  });

  if (connectedPeer) {
    const hasConnectedDevice = [...deviceMap.values()].some(
      (device) => device.name === connectedPeer.name || device.peerId === connectedPeer.peerId,
    );

    if (!hasConnectedDevice) {
      deviceMap.set(
        connectedPeer.peerId ?? connectedPeer.name,
        {
          id: createDeviceId("connected", connectedPeer.peerId ?? connectedPeer.name),
          name: connectedPeer.name,
          os: connectedPeer.os,
          peerId: connectedPeer.peerId,
          address: connectedPeer.address,
          status: "connected",
          lastSeen: new Date(),
          source: "connected",
        },
      );
    }
  }

  return [...deviceMap.values()].sort((left, right) => {
    if (left.status !== right.status) {
      return left.status === "connected" ? -1 : 1;
    }
    return left.name.localeCompare(right.name, "zh-CN");
  });
}

function OsIcon({ os, size = 14 }: { os: OS; size?: number }) {
  return os === "macos" ? <Apple size={size} className="shrink-0" /> : <Monitor size={size} className="shrink-0" />;
}

function StatusDot({ status }: { status: DeviceStatus }) {
  const className =
    status === "connected"
      ? "bg-emerald-400 shadow-[0_0_6px_#34d399]"
      : status === "idle"
        ? "bg-amber-400 shadow-[0_0_6px_#fbbf24]"
        : "bg-zinc-600";

  return <span className={`inline-block h-1.5 w-1.5 shrink-0 rounded-full ${className}`} />;
}

function ClipTypeIcon({ type }: { type: ClipType }) {
  if (type === "image") {
    return (
      <span className="shrink-0 rounded bg-violet-400/10 p-1.5 text-violet-400">
        <Image size={14} />
      </span>
    );
  }

  if (type === "file") {
    return (
      <span className="shrink-0 rounded bg-amber-400/10 p-1.5 text-amber-400">
        <File size={14} />
      </span>
    );
  }

  return (
    <span className="shrink-0 rounded bg-primary/10 p-1.5 text-primary">
      <FileText size={14} />
    </span>
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  return (
    <button
      onClick={() => {
        navigator.clipboard.writeText(text).catch(() => undefined);
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1_600);
      }}
      className="rounded p-1.5 text-muted-foreground transition-colors hover:bg-primary/10 hover:text-primary"
      title="复制内容"
      type="button"
    >
      {copied ? <ShieldCheck size={13} /> : <Copy size={13} />}
    </button>
  );
}

function SettingBadge({ availability }: { availability: SettingAvailability }) {
  const config =
    availability === "editable"
      ? {
          label: "可调整",
          className: "border-primary/30 bg-primary/10 text-primary",
        }
      : availability === "managed"
        ? {
            label: "系统内置",
            className: "border-emerald-400/30 bg-emerald-400/10 text-emerald-400",
          }
        : {
            label: "暂不支持",
            className: "border-border bg-secondary text-muted-foreground",
          };

  return <span className={`shrink-0 rounded-full border px-2.5 py-1 text-[10px] font-medium ${config.className}`}>{config.label}</span>;
}

function ThemeSwatch({
  currentTheme,
  selectedTheme,
  isDark,
  onChange,
}: {
  currentTheme: ThemeColor;
  selectedTheme: ThemeColor;
  isDark: boolean;
  onChange: (theme: ThemeColor) => void;
}) {
  const color = isDark ? currentTheme.dark.primary : currentTheme.light.primary;

  return (
    <button
      onClick={() => onChange(currentTheme)}
      className="group flex flex-col items-center gap-1.5"
      title={currentTheme.label}
      type="button"
    >
      <span
        className={`h-6 w-6 rounded-full border-2 transition-transform group-hover:scale-110 ${selectedTheme.id === currentTheme.id ? "scale-110 border-foreground" : "border-transparent"}`}
        style={{ background: color, boxShadow: selectedTheme.id === currentTheme.id ? `0 0 8px ${color}` : "none" }}
      />
      <span className={`font-mono text-[10px] ${selectedTheme.id === currentTheme.id ? "text-primary" : "text-muted-foreground"}`}>
        {currentTheme.label}
      </span>
    </button>
  );
}

function Sidebar({
  activeNav,
  devices,
  status,
  identityLabel,
  colorScheme,
  setColorScheme,
  theme,
  isDark,
  onThemeChange,
  onNavigate,
}: {
  activeNav: NavId;
  devices: Device[];
  status: AppConnectionStatus;
  identityLabel: string;
  colorScheme: ColorScheme;
  setColorScheme: (scheme: ColorScheme) => void;
  theme: ThemeColor;
  isDark: boolean;
  onThemeChange: (theme: ThemeColor) => void;
  onNavigate: (nav: NavId) => void;
}) {
  const navItems = [
    { id: "clipboard" as const, label: "剪贴板", icon: <Clipboard size={15} /> },
    { id: "devices" as const, label: "设备", icon: <Radio size={15} /> },
    { id: "settings" as const, label: "设置", icon: <Settings size={15} /> },
  ];

  const statusLabel =
    status === "connecting" ? "连接中…" : status === "online" ? "已连接" : TAURI_AVAILABLE ? "监听中" : "预览模式";

  const statusClassName =
    status === "connecting"
      ? "bg-primary animate-pulse"
      : status === "online"
        ? "bg-emerald-400"
        : TAURI_AVAILABLE
          ? "bg-amber-400"
          : "bg-zinc-500";

  return (
    <aside className="flex h-full w-52 shrink-0 flex-col border-r border-border bg-card xl:w-56">
      <div className="border-b border-border px-4 pb-4 pt-5">
        <div className="flex items-center gap-2.5">
          <div className="flex h-7 w-7 items-center justify-center rounded-md bg-primary/15">
            <Clipboard size={14} className="text-primary" />
          </div>
          <div>
            <p className="text-sm font-semibold leading-none tracking-tight text-foreground">PlanarClip</p>
            <p className="mt-0.5 font-mono text-[10px] text-muted-foreground">{identityLabel}</p>
          </div>
        </div>
      </div>

      <nav className="space-y-0.5 px-2 pb-2 pt-3">
        {navItems.map((item) => (
          <button
            key={item.id}
            onClick={() => onNavigate(item.id)}
            className={`flex w-full items-center gap-2.5 rounded px-2.5 py-2 text-sm transition-colors ${
              activeNav === item.id
                ? "bg-primary/10 text-primary"
                : "text-muted-foreground hover:bg-secondary hover:text-foreground"
            }`}
            type="button"
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </nav>

      <div className="px-4 pb-2 pt-4">
        <p className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">局域网设备</p>
        {devices.length > 0 ? (
          <div className="space-y-1.5">
            {devices.map((device) => (
              <div key={device.id} className="flex items-center justify-between gap-2">
                <div className="flex min-w-0 items-center gap-2">
                  <StatusDot status={device.status} />
                  <span className="truncate text-xs text-foreground">{device.name}</span>
                </div>
                <span className="shrink-0 font-mono text-[10px] text-muted-foreground">
                  {device.status === "connected" ? "在线" : "可发现"}
                </span>
              </div>
            ))}
          </div>
        ) : (
          <div className="rounded-lg border border-dashed border-border px-3 py-3 text-xs text-muted-foreground">
            暂时还没有发现附近设备
          </div>
        )}
      </div>

      <div className="mt-auto space-y-3 border-t border-border px-3 pb-3 pt-3">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-muted-foreground">背景</span>
          <div className="flex items-center rounded-md bg-secondary p-0.5">
            {[
              { id: "light" as const, label: "浅色", icon: <Sun size={13} /> },
              { id: "dark" as const, label: "深色", icon: <Moon size={13} /> },
              { id: "system" as const, label: "跟随系统", icon: <SunMoon size={13} /> },
            ].map((option) => (
              <button
                key={option.id}
                onClick={() => setColorScheme(option.id)}
                title={option.label}
                className={`rounded p-1.5 transition-colors ${colorScheme === option.id ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
                type="button"
              >
                {option.icon}
              </button>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <div className={`h-1.5 w-1.5 shrink-0 rounded-full ${statusClassName}`} />
          <span className="text-[11px] text-muted-foreground">{statusLabel}</span>
          <div className="ml-auto flex items-center gap-2">
            <Palette size={12} className="text-muted-foreground" />
            <div className="flex items-center gap-2">
              {THEME_COLORS.map((currentTheme) => (
                <ThemeSwatch
                  key={currentTheme.id}
                  currentTheme={currentTheme}
                  selectedTheme={theme}
                  isDark={isDark}
                  onChange={onThemeChange}
                />
              ))}
            </div>
          </div>
        </div>
      </div>
    </aside>
  );
}

function ClipboardPage({
  clips,
  devices,
  viewMode,
  setViewMode,
  status,
  statusMessage,
}: {
  clips: ClipEntry[];
  devices: Device[];
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
  status: AppConnectionStatus;
  statusMessage: string;
}) {
  useRelativeTicker();
  const connectedCount = devices.filter((device) => device.status === "connected").length;

  return (
    <div className="flex min-w-0 flex-1 flex-col overflow-y-auto">
      <div className="flex shrink-0 items-center justify-between border-b border-border px-4 pb-3 pt-5 md:px-6">
        <div>
          <h1 className="text-base font-semibold text-foreground">剪贴板历史</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">最近 {clips.length} 条文本同步摘要</p>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center rounded-md bg-secondary p-0.5">
            <button
              onClick={() => setViewMode("list")}
              className={`rounded p-1.5 transition-colors ${viewMode === "list" ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
              title="列表视图"
              type="button"
            >
              <LayoutList size={14} />
            </button>
            <button
              onClick={() => setViewMode("grid")}
              className={`rounded p-1.5 transition-colors ${viewMode === "grid" ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
              title="网格视图"
              type="button"
            >
              <LayoutGrid size={14} />
            </button>
          </div>
          <div className="flex items-center gap-1.5 rounded bg-secondary px-2.5 py-1.5 font-mono text-[11px] text-muted-foreground">
            <Shield size={11} className="text-emerald-400" />
            已连接 {connectedCount} 台
          </div>
        </div>
      </div>

      {clips.length === 0 ? (
        <div className="flex flex-1 items-center justify-center px-6 py-10">
          <div className="max-w-sm rounded-2xl border border-dashed border-border bg-card/80 px-6 py-8 text-center">
            <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary/10 text-primary">
              <Clipboard size={24} />
            </div>
            <p className="text-sm font-medium text-foreground">
              {status === "online" ? "等待新的文本同步" : "连接建立后，这里会显示最近的文本同步摘要"}
            </p>
            <p className="mt-2 text-xs leading-6 text-muted-foreground">{statusMessage}</p>
          </div>
        </div>
      ) : viewMode === "list" ? (
        <div className="flex-1">
          {clips.map((clip) => {
            const sourceLine = clip.direction === "received" ? `来自 ${clip.sourceLabel}` : `从 ${clip.sourceLabel} 发出`;
            const statusLine = clip.direction === "received" ? "已写入这台设备的剪贴板" : "已广播到当前连接会话";

            return (
              <div key={clip.id} className="group border-b border-border px-4 py-4 transition-colors last:border-0 hover:bg-secondary/40 md:px-6">
                <div className="flex items-start gap-3">
                  <ClipTypeIcon type={clip.type} />
                  <div className="min-w-0 flex-1">
                    <div className="mb-1.5 flex items-center gap-2">
                      <span className="font-mono text-[11px] text-primary/80">{sourceLine}</span>
                      <span className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground">{relativeTime(clip.timestamp)}</span>
                      <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{clip.size}</span>
                      <div className="opacity-0 transition-opacity group-hover:opacity-100">
                        <CopyButton text={clip.content} />
                      </div>
                    </div>
                    <p className="line-clamp-3 whitespace-pre-wrap break-all font-mono text-sm leading-relaxed text-foreground/85">
                      {clip.content}
                    </p>
                    <div className="mt-2 flex items-center gap-1 font-mono text-[10px] text-emerald-400/70">
                      <Zap size={9} />
                      {statusLine}
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="grid content-start gap-3 p-4 md:p-5 xl:grid-cols-3 2xl:grid-cols-4">
          {clips.map((clip) => {
            const sourceLine = clip.direction === "received" ? `来自 ${clip.sourceLabel}` : `从 ${clip.sourceLabel} 发出`;

            return (
              <div key={clip.id} className="group flex flex-col overflow-hidden rounded-xl border border-border bg-card transition-colors hover:border-primary/30">
                <div className="flex flex-1 flex-col gap-2 p-3">
                  <div className="flex items-center gap-2">
                    <ClipTypeIcon type={clip.type} />
                    <span className="truncate font-mono text-[11px] text-primary/80">{sourceLine}</span>
                    <div className="ml-auto opacity-0 transition-opacity group-hover:opacity-100">
                      <CopyButton text={clip.content} />
                    </div>
                  </div>
                  <p className="line-clamp-4 flex-1 whitespace-pre-wrap break-all font-mono text-xs leading-relaxed text-foreground/80">
                    {clip.content}
                  </p>
                  <div className="mt-auto flex items-center justify-between border-t border-border pt-2">
                    <span className="font-mono text-[10px] text-muted-foreground">{relativeTime(clip.timestamp)}</span>
                    <span className="font-mono text-[10px] text-muted-foreground">{clip.size}</span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function DevicesPanel({
  devices,
  pairingCode,
  status,
}: {
  devices: Device[];
  pairingCode: string;
  status: AppConnectionStatus;
}) {
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

function DevicesPage({
  devices,
  connectionStatus,
  onShowPairing,
  onConnectDevice,
  onDisconnect,
}: {
  devices: Device[];
  connectionStatus: AppConnectionStatus;
  onShowPairing: () => void;
  onConnectDevice: (device: Device) => void;
  onDisconnect: () => void;
}) {
  const busyConnecting = connectionStatus === "connecting";
  const hasActiveSession = connectionStatus === "online";

  return (
    <div className="flex-1 overflow-y-auto px-4 pt-6 md:px-6 md:pt-8 xl:px-8">
      <div className="mb-6 flex max-w-3xl items-start justify-between gap-4">
        <div>
          <h2 className="mb-1 text-base font-semibold text-foreground">设备管理</h2>
          <p className="text-sm text-muted-foreground">查看真实局域网发现结果，并直接发起连接或断开当前会话。</p>
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
        <p className="mb-3 font-mono text-[11px] uppercase tracking-widest text-muted-foreground">已发现设备</p>
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
                <p className="font-mono text-[11px] text-muted-foreground">{device.address}</p>
              </div>
              <div className="flex items-center gap-2">
                <StatusDot status={device.status} />
                <span className="text-xs text-muted-foreground">
                  {device.status === "connected" ? "已连接" : device.status === "idle" ? "可连接" : "离线"}
                </span>
              </div>
              {device.status === "connected" ? (
                <button
                  onClick={onDisconnect}
                  className="shrink-0 rounded-lg border border-border px-3 py-1.5 text-[11px] font-medium text-foreground transition-colors hover:border-destructive/30 hover:bg-destructive/10 hover:text-destructive"
                  type="button"
                >
                  断开连接
                </button>
              ) : (
                <button
                  onClick={() => onConnectDevice(device)}
                  disabled={connectDisabled}
                  className="shrink-0 rounded-lg bg-primary px-3 py-1.5 text-[11px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
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
            <p className="text-sm">暂无可连接设备</p>
            <p className="mt-1 text-xs">保持双方在同一局域网，并确认对方应用已打开</p>
          </div>
        )}
      </div>
    </div>
  );
}

function SettingsPage({
  colorScheme,
  isDark,
  theme,
  settingsMessage,
  isSaving,
  onSchemeChange,
  onThemeChange,
}: {
  colorScheme: ColorScheme;
  isDark: boolean;
  theme: ThemeColor;
  settingsMessage: string;
  isSaving: boolean;
  onSchemeChange: (scheme: ColorScheme) => void;
  onThemeChange: (theme: ThemeColor) => void;
}) {
  const settingRows: Array<{
    label: string;
    desc: string;
    availability: SettingAvailability;
  }> = [
    {
      label: "剪贴板变化时自动同步",
      desc: "文本链路当前始终保持自动同步，暂不提供关闭开关。",
      availability: "managed",
    },
    {
      label: "同步图片",
      desc: "当前版本只支持文本同步，图片能力会在后续阶段补齐。",
      availability: "planned",
    },
    {
      label: "同步文件",
      desc: "当前版本只支持文本同步，文件能力会在后续阶段补齐。",
      availability: "planned",
    },
    {
      label: "加密传输内容",
      desc: "当前直连链路默认启用加密传输，无需额外设置。",
      availability: "managed",
    },
    {
      label: "显示接收通知",
      desc: "提醒能力还没有接到真实桌面通知链路，当前先保留为后续能力。",
      availability: "planned",
    },
  ];

  return (
    <div className="max-w-3xl flex-1 overflow-y-auto px-4 pt-6 md:px-6 md:pt-8 xl:px-8">
      <h2 className="mb-1 text-base font-semibold text-foreground">设置</h2>
      <p className="mb-6 text-sm text-muted-foreground">管理已落地的外观项，并查看当前版本可用的同步能力边界。</p>

      <div className="mb-6">
        <p className="mb-3 text-xs font-medium uppercase tracking-widest text-muted-foreground">外观</p>
        <div className="space-y-5 rounded-xl border border-border bg-card p-4">
          <div>
            <div className="mb-3 flex items-start justify-between gap-4">
              <div>
                <p className="mb-0.5 text-sm font-medium text-foreground">背景模式</p>
                <p className="text-xs text-muted-foreground">设置界面的明暗风格，并同步保存到桌面端配置。</p>
              </div>
              <SettingBadge availability="editable" />
            </div>
            <div className="flex gap-2">
              {[
                { id: "light" as const, label: "浅色", icon: <Sun size={14} /> },
                { id: "dark" as const, label: "深色", icon: <Moon size={14} /> },
                { id: "system" as const, label: "跟随系统", icon: <SunMoon size={14} /> },
              ].map((item) => (
                <button
                  key={item.id}
                  onClick={() => onSchemeChange(item.id)}
                  className={`flex flex-1 flex-col items-center gap-1.5 rounded-lg border py-3 transition-colors ${
                    colorScheme === item.id
                      ? "border-primary bg-primary/10 text-primary"
                      : "border-border text-muted-foreground hover:border-primary/40 hover:text-foreground"
                  }`}
                  type="button"
                >
                  {item.icon}
                  <span className="text-[11px] font-medium">{item.label}</span>
                </button>
              ))}
            </div>
          </div>

          <div>
            <div className="mb-3 flex items-start justify-between gap-4">
              <div>
                <p className="mb-0.5 text-sm font-medium text-foreground">主题色</p>
                <p className="text-xs text-muted-foreground">切换界面强调色，并在下次打开时继续沿用。</p>
              </div>
              <SettingBadge availability="editable" />
            </div>
            <div className="flex items-center gap-4">
              {THEME_COLORS.map((currentTheme) => (
                <ThemeSwatch
                  key={currentTheme.id}
                  currentTheme={currentTheme}
                  selectedTheme={theme}
                  isDark={isDark}
                  onChange={onThemeChange}
                />
              ))}
            </div>
          </div>

          <div className="rounded-lg border border-dashed border-border bg-secondary/20 px-3 py-3 text-xs text-muted-foreground">
            {isSaving ? "正在保存外观设置…" : settingsMessage}
          </div>
        </div>
      </div>

      <p className="mb-3 text-xs font-medium uppercase tracking-widest text-muted-foreground">同步与安全</p>
      <div className="rounded-xl border border-border bg-card px-4">
        {settingRows.map((item) => (
          <div key={item.label} className="flex items-start justify-between gap-4 border-b border-border py-3.5 last:border-0">
            <div>
              <p className="text-sm font-medium text-foreground">{item.label}</p>
              <p className="mt-0.5 text-xs leading-6 text-muted-foreground">{item.desc}</p>
            </div>
            <SettingBadge availability={item.availability} />
          </div>
        ))}
      </div>
    </div>
  );
}

function PairingModal({
  pairingCode,
  input,
  stage,
  discoveredDevices,
  helperText,
  errorMessage,
  incomingRequest,
  onClose,
  onInputChange,
  onManualPair,
  onConnectLan,
  onSubmitPairingCode,
  onRejectIncoming,
}: {
  pairingCode: string;
  input: string;
  stage: PairingStage;
  discoveredDevices: Device[];
  helperText: string;
  errorMessage: string | null;
  incomingRequest: ConnectionRequestPayload | null;
  onClose: () => void;
  onInputChange: (value: string) => void;
  onManualPair: () => void;
  onConnectLan: (device: Device) => void;
  onSubmitPairingCode: () => void;
  onRejectIncoming: () => void;
}) {
  const statusConfig =
    stage === "manual_pairing" || stage === "requesting_device" || stage === "submitting_code"
      ? {
          icon: <Loader2 size={14} className="shrink-0 animate-spin text-primary" />,
          text: "正在建立连接…",
          className: "text-primary",
        }
      : stage === "awaiting_code"
        ? {
            icon: <ShieldCheck size={14} className="shrink-0 text-amber-400" />,
            text: "等待输入配对码",
            className: "text-amber-400",
          }
        : stage === "incoming_request"
          ? {
              icon: <Wifi size={14} className="shrink-0 text-primary" />,
              text: "收到新的连接请求",
              className: "text-primary",
            }
          : stage === "error"
            ? {
                icon: <WifiOff size={14} className="shrink-0 text-destructive" />,
                text: errorMessage ?? "连接未完成，请重新尝试。",
                className: "text-destructive",
              }
            : {
                icon: <WifiOff size={14} className="shrink-0 text-muted-foreground" />,
                text: "未连接",
                className: "text-muted-foreground",
              };

  const submitting =
    stage === "manual_pairing" || stage === "requesting_device" || stage === "submitting_code";
  const submitLabel =
    stage === "submitting_code"
      ? "正在提交…"
      : stage === "awaiting_code"
        ? "提交配对码"
        : stage === "manual_pairing"
          ? "正在校验…"
          : "发起配对";
  const deviceActionLabel = stage === "requesting_device" ? "请求中…" : "连接设备";
  const closeLabel = submitting ? "取消这次连接" : "关闭";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative mx-4 w-full max-w-sm overflow-hidden rounded-2xl border border-border bg-card shadow-2xl">
        <div className="flex items-center justify-between border-b border-border px-5 pb-4 pt-5">
          <div>
            <p className="text-sm font-semibold text-foreground">连接新设备</p>
            <p className="mt-0.5 text-xs text-muted-foreground">通过配对码与另一台设备建立连接</p>
          </div>
          <button
            onClick={onClose}
            title={closeLabel}
            className="rounded-lg p-1.5 text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            type="button"
          >
            <X size={15} />
          </button>
        </div>

        <div className="max-h-[80vh] space-y-5 overflow-y-auto p-5">
          <div className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3">
            {statusConfig.icon}
            <span className={`text-xs ${statusConfig.className}`}>{statusConfig.text}</span>
          </div>

          <div className="text-center">
            <p className="mb-3 font-mono text-[11px] uppercase tracking-widest text-muted-foreground">你的配对码</p>
            <div className="mb-3 flex items-center justify-center gap-2">
              {pairingCode.split("").map((digit, index) => (
                <span
                  key={`${digit}-${index}`}
                  className="flex h-12 w-10 items-center justify-center rounded-lg border border-border bg-secondary font-mono text-2xl font-bold text-primary"
                >
                  {digit}
                </span>
              ))}
            </div>
            <p className="text-[11px] text-muted-foreground">请在另一台设备上输入此配对码</p>
          </div>

          {incomingRequest ? (
            <div className="space-y-3 rounded-xl border border-primary/20 bg-primary/5 p-4">
              <div>
                <p className="text-sm font-medium text-foreground">{incomingRequest.device_name} 想要连接这台设备</p>
                <p className="mt-1 text-xs leading-6 text-muted-foreground">
                  请在对方设备上输入下方配对码；如果这不是你发起的连接，可以直接拒绝。
                </p>
              </div>
              <div className="rounded-lg border border-border bg-card px-3 py-2.5 font-mono text-center text-lg tracking-[0.3em] text-primary">
                {incomingRequest.pairing_code}
              </div>
              <button
                onClick={onRejectIncoming}
                className="w-full rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm font-medium text-destructive transition-colors hover:bg-destructive/15"
                type="button"
              >
                拒绝这次连接
              </button>
            </div>
          ) : (
            <>
              <div className="flex items-center gap-3">
                <div className="h-px flex-1 bg-border" />
                <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">或</span>
                <div className="h-px flex-1 bg-border" />
              </div>

              <div>
                <p className="mb-2 font-mono text-[11px] uppercase tracking-widest text-muted-foreground">输入对方设备上的配对码</p>
                <div className="flex gap-2">
                  <input
                    type="text"
                    inputMode="numeric"
                    maxLength={6}
                    placeholder="000000"
                    value={input}
                    onChange={(event) => onInputChange(event.target.value.replace(/\D/g, "").slice(0, 6))}
                    disabled={submitting}
                    className="flex-1 rounded-lg border border-border bg-secondary px-3 py-2.5 text-center font-mono text-lg tracking-[0.25em] text-foreground transition-colors placeholder:text-muted-foreground/50 focus:border-primary focus:outline-none disabled:opacity-50"
                  />
                  <button
                    onClick={stage === "awaiting_code" ? onSubmitPairingCode : onManualPair}
                    disabled={input.length !== 6 || submitting}
                    className="rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                    type="button"
                  >
                    {submitting ? <Loader2 size={15} className="animate-spin" /> : submitLabel}
                  </button>
                </div>
                <p className={`mt-2 text-xs ${errorMessage ? "text-destructive" : "text-muted-foreground"}`}>
                  {errorMessage ?? helperText}
                </p>
              </div>

              <div>
                <div className="mb-2 flex items-center justify-between">
                  <p className="font-mono text-[11px] uppercase tracking-widest text-muted-foreground">局域网设备</p>
                  <span className="flex items-center gap-1 text-[10px] text-muted-foreground">
                    <Wifi size={10} />
                    自动发现
                  </span>
                </div>
                <div className="space-y-2">
                  {discoveredDevices.length > 0 ? (
                    discoveredDevices.map((device) => (
                      <div
                        key={device.id}
                        className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3 transition-colors hover:border-primary/30"
                      >
                        <div className="rounded-lg bg-primary/10 p-1.5 text-primary">
                          <OsIcon os={device.os} size={14} />
                        </div>
                        <div className="min-w-0 flex-1">
                          <p className="text-xs font-medium text-foreground">{device.name}</p>
                          <p className="font-mono text-[10px] text-muted-foreground">{device.address}</p>
                        </div>
                        <button
                          onClick={() => onConnectLan(device)}
                          disabled={submitting}
                          className="shrink-0 rounded-lg bg-primary px-3 py-1.5 text-[11px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                          type="button"
                        >
                          {deviceActionLabel}
                        </button>
                      </div>
                    ))
                  ) : (
                    <div className="rounded-xl border border-dashed border-border px-3 py-5 text-center text-xs text-muted-foreground">
                      暂时还没有发现附近设备
                    </div>
                  )}
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

export default function App() {
  const previewUiSettings = useMemo(() => loadPreviewUiSettings(), []);
  const [activeNav, setActiveNav] = useState<NavId>("clipboard");
  const [clips, setClips] = useState<ClipEntry[]>(EMPTY_CLIPS);
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [theme, setTheme] = useState<ThemeColor>(() => getThemeById(previewUiSettings.theme_color));
  const [colorScheme, setColorScheme] = useState<ColorScheme>(() => normalizeColorScheme(previewUiSettings.color_scheme));
  const [isDark, setIsDark] = useState(true);
  const [settingsMessage, setSettingsMessage] = useState(
    TAURI_AVAILABLE ? "正在同步桌面端设置…" : "当前是浏览器预览模式，外观设置会暂存到浏览器本地。",
  );
  const [isSavingSettings, setIsSavingSettings] = useState(false);
  const [status, setStatus] = useState<AppConnectionStatus>(TAURI_AVAILABLE ? "connecting" : "offline");
  const [lastMessage, setLastMessage] = useState(
    TAURI_AVAILABLE ? "正在初始化连接桥接…" : "当前是浏览器预览模式，连接能力需在桌面应用中体验。",
  );
  const [pairingCode, setPairingCode] = useState("------");
  const [lanDevices, setLanDevices] = useState<LanDevicePayload[]>([]);
  const [connectedPeer, setConnectedPeer] = useState<ConnectedPeer | null>(null);
  const [showPairing, setShowPairing] = useState(false);
  const [pairingInput, setPairingInput] = useState("");
  const [pairingStage, setPairingStage] = useState<PairingStage>("idle");
  const [pairingTargetName, setPairingTargetName] = useState<string | null>(null);
  const [pairingHelperText, setPairingHelperText] = useState("通过配对码或局域网设备建立连接。");
  const [pairingError, setPairingError] = useState<string | null>(null);
  const [incomingRequest, setIncomingRequest] = useState<ConnectionRequestPayload | null>(null);

  const statusRef = useRef(status);
  const pairingStageRef = useRef(pairingStage);
  const connectedPeerRef = useRef(connectedPeer);
  const pairingTargetRef = useRef<string | null>(pairingTargetName);

  const devices = useMemo(() => buildDevices(lanDevices, connectedPeer), [lanDevices, connectedPeer]);
  const discoveredDevices = useMemo(() => devices.filter((device) => device.source === "discovery"), [devices]);
  const identityLabel = pairingCode === "------" ? "连接信息准备中" : `配对码 ${pairingCode}`;

  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  useEffect(() => {
    pairingStageRef.current = pairingStage;
  }, [pairingStage]);

  useEffect(() => {
    connectedPeerRef.current = connectedPeer;
  }, [connectedPeer]);

  useEffect(() => {
    pairingTargetRef.current = pairingTargetName;
  }, [pairingTargetName]);

  const syncTheme = useCallback(
    (nextTheme: ThemeColor, nextScheme: ColorScheme) => {
      applyColorScheme(nextScheme);
      applyThemeColor(nextTheme);
      setIsDark(isDarkActive());
    },
    [],
  );

  const persistUiSettings = useCallback(async (nextScheme: ColorScheme, nextTheme: ThemeColor) => {
    if (!TAURI_AVAILABLE) {
      savePreviewUiSettings({
        color_scheme: nextScheme,
        theme_color: nextTheme.id,
      });
      setSettingsMessage("当前是浏览器预览模式，外观设置已暂存到浏览器本地。");
      return;
    }

    setIsSavingSettings(true);
    try {
      await callCommand<UiSettingsPayload>("save_ui_settings", {
        colorScheme: nextScheme,
        themeColor: nextTheme.id,
      });
      setSettingsMessage("外观设置已保存，下次打开桌面应用时会继续保留。");
    } catch (error) {
      setSettingsMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
    } finally {
      setIsSavingSettings(false);
    }
  }, []);

  const handleColorSchemeChange = useCallback(
    (nextScheme: ColorScheme) => {
      setColorScheme(nextScheme);
      void persistUiSettings(nextScheme, theme);
    },
    [persistUiSettings, theme],
  );

  const handleThemeChange = useCallback(
    (nextTheme: ThemeColor) => {
      setTheme(nextTheme);
      void persistUiSettings(colorScheme, nextTheme);
    },
    [colorScheme, persistUiSettings],
  );

  const resetPairingFlow = useCallback((closeModal = false) => {
    setPairingInput("");
    setPairingStage("idle");
    setPairingTargetName(null);
    setPairingHelperText("通过配对码或局域网设备建立连接。");
    setPairingError(null);
    setIncomingRequest(null);
    if (closeModal) {
      setShowPairing(false);
    }
  }, []);

  const closePairingModal = useCallback(async () => {
    if (incomingRequest) {
      try {
        await callCommand("reject_connection");
        setLastMessage("已拒绝这次连接请求。",);
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "拒绝连接时出了点问题，请稍后再试。"));
      }
      resetPairingFlow(true);
      setStatus("offline");
      return;
    }

    if (pairingStage === "awaiting_code" || pairingStage === "requesting_device" || pairingStage === "submitting_code") {
      try {
        await callCommand("disconnect");
      } catch {
        // 这里不额外提示，关闭弹层的目标是尽快回到可重试状态。
      }
      setLastMessage("已取消本次连接，你可以重新选择附近设备。",);
      setStatus("offline");
      resetPairingFlow(true);
      return;
    }

    resetPairingFlow(true);
  }, [incomingRequest, pairingStage, resetPairingFlow]);

  const refreshConnectionStatus = useCallback(async () => {
    try {
      const connectionStatus = await callCommand<string>("get_status");
      const isConnected = connectionStatus === "connected";

      if (isConnected) {
        if (statusRef.current !== "online") {
          setStatus("online");
        }
        return;
      }

      if (pairingStageRef.current === "idle") {
        if (connectedPeerRef.current) {
          setConnectedPeer(null);
          setLastMessage("当前连接已断开，请重新连接。",);
        }
        setStatus("offline");
      }
    } catch {
      if (pairingStageRef.current === "idle") {
        setStatus("offline");
      }
    }
  }, []);

  const handleManualPair = useCallback(async () => {
    if (pairingInput.length !== 6) {
      setPairingError("请输入 6 位数字配对码。");
      return;
    }

    setPairingStage("manual_pairing");
    setPairingError(null);
    setPairingHelperText(`正在根据配对码 ${pairingInput} 建立连接…`);
    setStatus("connecting");
    setLastMessage(`正在根据配对码 ${pairingInput} 建立连接…`);

    try {
      await callCommand<string>("pair", { code: pairingInput });
      setConnectedPeer({
        name: "已配对设备",
        address: `配对码 ${pairingInput}`,
        os: "windows",
        source: "pair",
      });
      setStatus("online");
      setLastMessage(`已完成配对，连接已建立 — ${formatTime()}`);
      resetPairingFlow(true);
    } catch (error) {
      const message = normalizeUserMessage(error, "这次配对没有成功，请稍后再试。");
      setPairingStage("error");
      setPairingError(message);
      setPairingHelperText(message);
      setStatus("offline");
      setLastMessage(message);
    }
  }, [pairingInput, resetPairingFlow]);

  const handleConnectLan = useCallback(
    async (device: Device) => {
      if (!device.host || !device.port) {
        setPairingError("当前设备缺少连接地址，请等待下一轮发现结果。",);
        return;
      }

      if (status === "online") {
        setPairingError("当前已经建立连接，如需切换设备，请先断开当前连接。",);
        return;
      }

      setShowPairing(true);
      setPairingTargetName(device.name);
      setPairingStage("requesting_device");
      setPairingError(null);
      setPairingHelperText(`正在请求连接 ${device.name}，请稍候…`);
      setStatus("connecting");
      setLastMessage(`正在请求连接 ${device.name}，请稍候…`);

      try {
        const result = await callCommand<string>("connect_lan", { ip: device.host, port: device.port });
        if (result === "awaiting_code") {
          const message = `请查看 ${device.name} 屏幕上的 6 位配对码，并在这里输入。`;
          setPairingStage("awaiting_code");
          setPairingHelperText(message);
          setLastMessage(message);
          return;
        }

        setConnectedPeer({
          name: device.name,
          peerId: device.peerId,
          address: device.address,
          os: device.os,
          source: "lan",
        });
        setStatus("online");
        setLastMessage(`已与 ${device.name} 建立连接，现在可以开始同步剪贴板了 — ${formatTime()}`);
        resetPairingFlow(true);
      } catch (error) {
        const message = normalizeUserMessage(error, `暂时无法连接 ${device.name}，请稍后重试。`, device.name);
        setPairingStage("error");
        setPairingError(message);
        setPairingHelperText(message);
        setStatus("offline");
        setLastMessage(message);
      }
    },
    [resetPairingFlow, status],
  );

  const handleSubmitPairingCode = useCallback(async () => {
    if (pairingInput.length !== 6) {
      setPairingError("请输入 6 位数字配对码。");
      return;
    }

    setPairingStage("submitting_code");
    setPairingError(null);
    setStatus("connecting");

    try {
      await callCommand<string>("submit_pairing_code", { code: pairingInput });
    } catch (error) {
      const message = normalizeUserMessage(
        error,
        "这次连接没有成功，请重新发起连接。",
        pairingTargetRef.current ?? undefined,
      );
      setPairingStage("error");
      setPairingError(message);
      setPairingHelperText(message);
      setStatus("offline");
      setLastMessage(message);
    }
  }, [pairingInput]);

  const handleRejectIncoming = useCallback(async () => {
    try {
      await callCommand("reject_connection");
      setLastMessage("已拒绝这次连接请求。",);
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "拒绝连接时出了点问题，请稍后再试。"));
    }
    resetPairingFlow(true);
    setStatus("offline");
  }, [resetPairingFlow]);

  const handleDisconnect = useCallback(async () => {
    try {
      await callCommand("disconnect");
      setConnectedPeer(null);
      setStatus("offline");
      setLastMessage("已断开当前连接。",);
      resetPairingFlow(true);
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "断开连接时出了点问题，请稍后再试。"));
    }
  }, [resetPairingFlow]);

  useEffect(() => {
    syncTheme(theme, colorScheme);
  }, [colorScheme, syncTheme, theme]);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => {
      if (colorScheme === "system") {
        syncTheme(theme, "system");
      }
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [colorScheme, syncTheme, theme]);

  useEffect(() => {
    if (!TAURI_AVAILABLE) {
      setPairingCode("桌面端可用");
      setStatus("offline");
      setLastMessage("当前是浏览器预览模式，连接能力需在桌面应用中体验。",);
      return;
    }

    let disposed = false;
    let eventCleanup: Array<() => void> = [];

    const setup = async () => {
      try {
        const [initialStatus, initialPairingCode, initialLanDevices, initialUiSettings, initialClipboardHistory] = await Promise.all([
          callCommand<string>("get_status"),
          callCommand<string>("get_pairing_code"),
          callCommand<LanDevicePayload[]>("get_lan_devices"),
          callCommand<UiSettingsPayload>("get_ui_settings"),
          callCommand<ClipboardHistoryPayload[]>("get_clipboard_history"),
        ]);

        if (disposed) {
          return;
        }

        setPairingCode(initialPairingCode);
        setLanDevices(initialLanDevices);
        setClips(mapClipboardHistory(initialClipboardHistory));
        setColorScheme(normalizeColorScheme(initialUiSettings.color_scheme));
        setTheme(getThemeById(initialUiSettings.theme_color));
        setSettingsMessage("桌面端设置已同步，可直接继续调整外观。");
        setStatus(initialStatus === "connected" ? "online" : "offline");
        setLastMessage(
          initialStatus === "connected"
            ? "已恢复现有连接，可以继续同步剪贴板。"
            : "正在监听局域网设备与连接请求。",
        );
      } catch (error) {
        if (!disposed) {
          setStatus("offline");
          setSettingsMessage("暂时还没有同步到桌面端设置，当前先使用默认外观。",);
          setLastMessage(normalizeUserMessage(error, "连接桥接初始化失败，请稍后再试。"));
        }
      }

      const listeners = await Promise.all([
        listen<LanDevicePayload[]>("lan-devices-changed", (event) => {
          setLanDevices(event.payload);
        }),
        listen<ClipboardHistoryPayload[]>("clipboard-history-changed", (event) => {
          setClips(mapClipboardHistory(event.payload));
        }),
        listen<ConnectionRequestPayload>("connection-request", (event) => {
          setIncomingRequest(event.payload);
          setPairingTargetName(event.payload.device_name);
          setPairingStage("incoming_request");
          setPairingError(null);
          setPairingHelperText(`请在 ${event.payload.device_name} 上输入下方配对码，或直接拒绝这次连接。`);
          setShowPairing(true);
          setStatus("connecting");
          setLastMessage(`${event.payload.device_name} 正在请求连接，请核对配对码后决定是否继续。`);
        }),
        listen<ConnectionEstablishedPayload>("connection-established", (event) => {
          setConnectedPeer({
            name: event.payload.peer_name || "已连接设备",
            peerId: event.payload.peer_id,
            address: pairingTargetRef.current ? `${pairingTargetRef.current} · 局域网直连` : "局域网直连",
            os: inferOs(event.payload.peer_name || "已连接设备"),
            source: "lan",
          });
          setStatus("online");
          setLastMessage(
            event.payload.is_reconnect
              ? `已恢复与 ${event.payload.peer_name} 的连接。`
              : `已与 ${event.payload.peer_name} 建立连接，现在可以开始同步剪贴板了 — ${formatTime()}`,
          );
          resetPairingFlow(true);
        }),
        listen<ConnectionFailedPayload>("connection-failed", (event) => {
          const message = normalizeUserMessage(
            event.payload,
            "这次连接没有成功，请重新发起连接。",
            pairingTargetRef.current ?? undefined,
          );
          setPairingStage("error");
          setPairingError(message);
          setPairingHelperText(message);
          setStatus("offline");
          setLastMessage(message);
        }),
        listen<ConnectionEndedPayload>("connection-ended", (event) => {
          const message = normalizeUserMessage(event.payload, "连接已断开，请重新连接。", event.payload.peer_name);
          setConnectedPeer(null);
          setStatus("offline");
          setLastMessage(message);
          resetPairingFlow(true);
        }),
        listen("pairing-code-needed", () => {
          const targetName = pairingTargetRef.current ?? "对方设备";
          const message = `请查看 ${targetName} 屏幕上的 6 位配对码，并在这里输入。`;
          setPairingStage("awaiting_code");
          setPairingError(null);
          setPairingHelperText(message);
          setShowPairing(true);
          setStatus("connecting");
          setLastMessage(message);
        }),
      ]);

      if (disposed) {
        listeners.forEach((unlisten) => unlisten());
        return;
      }

      eventCleanup = listeners;
    };

    void setup();

    const timer = window.setInterval(() => {
      void refreshConnectionStatus();
    }, 5_000);

    return () => {
      disposed = true;
      window.clearInterval(timer);
      eventCleanup.forEach((unlisten) => unlisten());
    };
  }, [refreshConnectionStatus, resetPairingFlow]);

  return (
    <div className="flex h-screen w-full overflow-hidden bg-background text-foreground" style={{ fontFamily: "'Plus Jakarta Sans', sans-serif" }}>
      <Sidebar
        activeNav={activeNav}
        devices={devices}
        status={status}
        identityLabel={identityLabel}
        colorScheme={colorScheme}
        setColorScheme={handleColorSchemeChange}
        theme={theme}
        isDark={isDark}
        onThemeChange={handleThemeChange}
        onNavigate={setActiveNav}
      />

      <main className="flex h-full min-w-0 flex-1 overflow-hidden">
        {activeNav === "clipboard" && (
          <>
            <ClipboardPage
              clips={clips}
              devices={devices}
              viewMode={viewMode}
              setViewMode={setViewMode}
              status={status}
              statusMessage={lastMessage}
            />
            <DevicesPanel devices={devices} pairingCode={pairingCode} status={status} />
          </>
        )}
        {activeNav === "devices" && (
          <DevicesPage
            devices={devices}
            connectionStatus={status}
            onShowPairing={() => setShowPairing(true)}
            onConnectDevice={(device) => {
              void handleConnectLan(device);
            }}
            onDisconnect={() => {
              void handleDisconnect();
            }}
          />
        )}
        {activeNav === "settings" && (
          <SettingsPage
            colorScheme={colorScheme}
            isDark={isDark}
            theme={theme}
            settingsMessage={settingsMessage}
            isSaving={isSavingSettings}
            onSchemeChange={handleColorSchemeChange}
            onThemeChange={handleThemeChange}
          />
        )}
      </main>

      {showPairing && (
        <PairingModal
          pairingCode={pairingCode.padEnd(6, "•").slice(0, 6)}
          input={pairingInput}
          stage={pairingStage}
          discoveredDevices={discoveredDevices}
          helperText={pairingHelperText}
          errorMessage={pairingError}
          incomingRequest={incomingRequest}
          onClose={() => {
            void closePairingModal();
          }}
          onInputChange={setPairingInput}
          onManualPair={() => {
            void handleManualPair();
          }}
          onConnectLan={(device) => {
            void handleConnectLan(device);
          }}
          onSubmitPairingCode={() => {
            void handleSubmitPairingCode();
          }}
          onRejectIncoming={() => {
            void handleRejectIncoming();
          }}
        />
      )}
    </div>
  );
}
