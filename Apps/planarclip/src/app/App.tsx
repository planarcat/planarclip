import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Apple,
  Check,
  Clipboard,
  Copy,
  File,
  FileText,
  Image,
  LayoutGrid,
  LayoutList,
  Loader2,
  Moon,
  Monitor,
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

type Device = {
  id: string;
  name: string;
  os: OS;
  ip: string;
  status: DeviceStatus;
  latency: number;
  lastSeen: Date;
};

type ClipEntry = {
  id: string;
  type: ClipType;
  content: string;
  preview?: string;
  sourceDeviceId: string;
  size: string;
  timestamp: Date;
};

type ThemeColor = {
  id: string;
  label: string;
  dark: { primary: string; accent: string; ring: string };
  light: { primary: string; accent: string; ring: string };
};

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

const DEVICES: Device[] = [
  {
    id: "dev-1",
    name: 'MacBook Pro 16"',
    os: "macos",
    ip: "192.168.1.101:19876",
    status: "connected",
    latency: 3,
    lastSeen: new Date(),
  },
  {
    id: "dev-2",
    name: "DESKTOP-K7MX9",
    os: "windows",
    ip: "192.168.1.104:19876",
    status: "connected",
    latency: 8,
    lastSeen: new Date(Date.now() - 15_000),
  },
  {
    id: "dev-3",
    name: "Surface Pro 9",
    os: "windows",
    ip: "192.168.1.117:19876",
    status: "idle",
    latency: 12,
    lastSeen: new Date(Date.now() - 95_000),
  },
];

const INITIAL_CLIPS: ClipEntry[] = [
  {
    id: "clip-1",
    type: "text",
    content:
      "const handleClipboardSync = async (payload: ClipboardPayload) => {\n  await broadcastToDevices(payload);\n};",
    sourceDeviceId: "dev-1",
    size: "128 B",
    timestamp: new Date(Date.now() - 25_000),
  },
  {
    id: "clip-2",
    type: "image",
    content: "screenshot_2026-06-21.png",
    preview:
      "https://images.unsplash.com/photo-1555066931-4365d14bab8c?w=400&h=240&fit=crop&auto=format",
    sourceDeviceId: "dev-2",
    size: "1.4 MB",
    timestamp: new Date(Date.now() - 74_000),
  },
  {
    id: "clip-3",
    type: "file",
    content: "design-specs-v3.figma",
    sourceDeviceId: "dev-3",
    size: "8.2 MB",
    timestamp: new Date(Date.now() - 430_000),
  },
  {
    id: "clip-4",
    type: "text",
    content: "会议纪要：使用 mDNS 完成局域网发现协议，对所有数据包实施 AES-256-GCM 加密。",
    sourceDeviceId: "dev-1",
    size: "210 B",
    timestamp: new Date(Date.now() - 180_000),
  },
];

const DISCOVERED_DEVICES = [
  { name: "My MacBook Air", ip: "192.168.1.108:19876", os: "macos" as const },
  { name: "DESKTOP-R3F2M", ip: "192.168.1.122:19876", os: "windows" as const },
];

function relativeTime(date: Date) {
  const diff = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (diff < 60) return `${diff}秒前`;
  if (diff < 3_600) return `${Math.floor(diff / 60)}分钟前`;
  if (diff < 86_400) return `${Math.floor(diff / 3_600)}小时前`;
  return `${Math.floor(diff / 86_400)}天前`;
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

  return <span className={`inline-block h-1.5 w-1.5 rounded-full shrink-0 ${className}`} />;
}

function ClipTypeIcon({ type }: { type: ClipType }) {
  if (type === "image") {
    return (
      <span className="rounded bg-violet-400/10 p-1.5 text-violet-400 shrink-0">
        <Image size={14} />
      </span>
    );
  }

  if (type === "file") {
    return (
      <span className="rounded bg-amber-400/10 p-1.5 text-amber-400 shrink-0">
        <File size={14} />
      </span>
    );
  }

  return (
    <span className="rounded bg-primary/10 p-1.5 text-primary shrink-0">
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
    >
      {copied ? <Check size={13} /> : <Copy size={13} />}
    </button>
  );
}

function ToggleSwitch({ defaultOn }: { defaultOn: boolean }) {
  const [enabled, setEnabled] = useState(defaultOn);

  return (
    <button
      onClick={() => setEnabled((value) => !value)}
      className={`relative h-5 w-9 shrink-0 rounded-full transition-colors ${enabled ? "bg-primary" : "bg-switch-background"}`}
      type="button"
    >
      <span
        className={`absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white transition-transform ${enabled ? "translate-x-4" : ""}`}
      />
    </button>
  );
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
  syncing,
  localIp,
  colorScheme,
  setColorScheme,
  theme,
  isDark,
  onThemeChange,
  onNavigate,
}: {
  activeNav: NavId;
  devices: Device[];
  syncing: boolean;
  localIp: string;
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

  return (
    <aside className="flex h-full w-56 shrink-0 flex-col border-r border-border bg-card">
      <div className="border-b border-border px-4 pb-4 pt-5">
        <div className="flex items-center gap-2.5">
          <div className="flex h-7 w-7 items-center justify-center rounded-md bg-primary/15">
            <Clipboard size={14} className="text-primary" />
          </div>
          <div>
            <p className="text-sm font-semibold leading-none tracking-tight text-foreground">PlanarClip</p>
            <p className="mt-0.5 font-mono text-[10px] text-muted-foreground">{localIp}</p>
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
        <div className="space-y-1.5">
          {devices.map((device) => (
            <div key={device.id} className="flex items-center justify-between">
              <div className="flex min-w-0 items-center gap-2">
                <StatusDot status={device.status} />
                <span className="truncate text-xs text-foreground">{device.name}</span>
              </div>
              <span className="ml-1 shrink-0 font-mono text-[10px] text-muted-foreground">{device.latency}ms</span>
            </div>
          ))}
        </div>
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
          <div className={`h-1.5 w-1.5 shrink-0 rounded-full ${syncing ? "bg-primary animate-pulse" : "bg-emerald-400"}`} />
          <span className="text-[11px] text-muted-foreground">{syncing ? "同步中…" : "监听中"}</span>
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
}: {
  clips: ClipEntry[];
  devices: Device[];
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
}) {
  useRelativeTicker();
  const deviceMap = useMemo(() => Object.fromEntries(devices.map((device) => [device.id, device])), [devices]);
  const connectedCount = devices.filter((device) => device.status === "connected").length;

  return (
    <div className="flex min-w-0 flex-1 flex-col overflow-y-auto">
      <div className="flex shrink-0 items-center justify-between border-b border-border px-6 pb-3 pt-5">
        <div>
          <h1 className="text-base font-semibold text-foreground">剪贴板历史</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">已同步至 {connectedCount} 台设备</p>
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
            AES-256
          </div>
        </div>
      </div>

      {viewMode === "list" ? (
        <div className="flex-1">
          {clips.map((clip) => {
            const sourceDevice = deviceMap[clip.sourceDeviceId] as Device | undefined;
            return (
              <div key={clip.id} className="group border-b border-border px-6 py-4 transition-colors last:border-0 hover:bg-secondary/40">
                <div className="flex items-start gap-3">
                  <ClipTypeIcon type={clip.type} />
                  <div className="min-w-0 flex-1">
                    <div className="mb-1.5 flex items-center gap-2">
                      {sourceDevice && (
                        <span className="flex items-center gap-1 font-mono text-[11px] text-primary/80">
                          <OsIcon os={sourceDevice.os} size={11} />
                          {sourceDevice.name}
                        </span>
                      )}
                      <span className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground">{relativeTime(clip.timestamp)}</span>
                      <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{clip.size}</span>
                      {clip.type === "text" && (
                        <div className="opacity-0 transition-opacity group-hover:opacity-100">
                          <CopyButton text={clip.content} />
                        </div>
                      )}
                    </div>
                    {clip.type === "image" && clip.preview ? (
                      <div className="mt-1.5 h-28 w-48 overflow-hidden rounded bg-muted">
                        <img src={clip.preview} alt={clip.content} className="h-full w-full object-cover" />
                      </div>
                    ) : clip.type === "file" ? (
                      <p className="font-mono text-sm text-amber-400/90">{clip.content}</p>
                    ) : (
                      <p className="line-clamp-3 whitespace-pre-wrap break-all font-mono text-sm leading-relaxed text-foreground/85">
                        {clip.content}
                      </p>
                    )}
                    <div className="mt-2 flex items-center gap-1 font-mono text-[10px] text-emerald-400/70">
                      <Zap size={9} />
                      已传输至 {Math.max(1, devices.length - 1)} 台设备
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="grid content-start gap-3 p-4 xl:grid-cols-3 2xl:grid-cols-4">
          {clips.map((clip) => {
            const sourceDevice = deviceMap[clip.sourceDeviceId] as Device | undefined;
            return (
              <div key={clip.id} className="group flex flex-col overflow-hidden rounded-xl border border-border bg-card transition-colors hover:border-primary/30">
                {clip.type === "image" && clip.preview && (
                  <div className="h-32 w-full bg-muted">
                    <img src={clip.preview} alt={clip.content} className="h-full w-full object-cover" />
                  </div>
                )}
                <div className="flex flex-1 flex-col gap-2 p-3">
                  <div className="flex items-center gap-2">
                    <ClipTypeIcon type={clip.type} />
                    {sourceDevice && (
                      <span className="flex items-center gap-1 truncate font-mono text-[11px] text-primary/80">
                        <OsIcon os={sourceDevice.os} size={11} />
                        {sourceDevice.name}
                      </span>
                    )}
                    {clip.type === "text" && (
                      <div className="ml-auto opacity-0 transition-opacity group-hover:opacity-100">
                        <CopyButton text={clip.content} />
                      </div>
                    )}
                  </div>
                  {clip.type === "text" ? (
                    <p className="line-clamp-4 flex-1 whitespace-pre-wrap break-all font-mono text-xs leading-relaxed text-foreground/80">
                      {clip.content}
                    </p>
                  ) : clip.type === "file" ? (
                    <p className="flex-1 truncate font-mono text-xs text-amber-400/90">{clip.content}</p>
                  ) : (
                    <p className="flex-1 truncate font-mono text-xs text-muted-foreground">{clip.content}</p>
                  )}
                  <div className="mt-auto flex items-center justify-between border-t border-border pt-2">
                    <span className="font-mono text-[10px] text-muted-foreground">{relativeTime(clip.timestamp)}</span>
                    <span className="flex items-center gap-1 font-mono text-[10px] text-emerald-400/70">
                      <Zap size={8} />→{Math.max(1, devices.length - 1)}台
                    </span>
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

function DevicesPanel({ devices }: { devices: Device[] }) {
  useRelativeTicker();

  return (
    <aside className="h-full w-64 shrink-0 overflow-y-auto border-l border-border bg-card">
      <div className="border-b border-border px-4 pb-3 pt-5">
        <p className="text-sm font-semibold text-foreground">已连接设备</p>
        <p className="mt-0.5 text-xs text-muted-foreground">局域网 · mDNS 发现</p>
      </div>
      <div className="space-y-2 p-3">
        {devices.map((device) => (
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
                { label: "IP 地址", value: device.ip, className: "text-foreground/80" },
                {
                  label: "延迟",
                  value: device.status === "offline" ? "—" : `${device.latency} ms`,
                  className:
                    device.latency < 10 ? "text-emerald-400" : device.latency < 25 ? "text-amber-400" : "text-red-400",
                },
                { label: "最近活跃", value: relativeTime(device.lastSeen), className: "text-foreground/70" },
              ].map((row) => (
                <div key={row.label} className="flex items-center justify-between">
                  <span className="text-[10px] text-muted-foreground">{row.label}</span>
                  <span className={`font-mono text-[10px] ${row.className}`}>{row.value}</span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
      <div className="mx-3 mb-3 rounded-lg border border-border bg-primary/5 p-3">
        <p className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">网络信息</p>
        <div className="space-y-1">
          {[
            { label: "子网", value: "192.168.1.0/24", className: "text-foreground/80" },
            { label: "端口", value: "19876", className: "text-foreground/80" },
            { label: "协议", value: "Direct TCP", className: "text-primary" },
            { label: "加密", value: "AES-256-GCM", className: "text-emerald-400" },
          ].map((row) => (
            <div key={row.label} className="flex justify-between">
              <span className="text-[10px] text-muted-foreground">{row.label}</span>
              <span className={`font-mono text-[10px] ${row.className}`}>{row.value}</span>
            </div>
          ))}
        </div>
      </div>
    </aside>
  );
}

function DevicesPage({ devices, onShowPairing, onRemoveDevice }: { devices: Device[]; onShowPairing: () => void; onRemoveDevice: (id: string) => void }) {
  return (
    <div className="flex-1 overflow-y-auto px-8 pt-8">
      <div className="mb-6 flex max-w-xl items-start justify-between">
        <div>
          <h2 className="mb-1 text-base font-semibold text-foreground">设备管理</h2>
          <p className="text-sm text-muted-foreground">管理局域网设备的配对与同步权限。</p>
        </div>
        <button
          onClick={onShowPairing}
          className="ml-4 flex shrink-0 items-center gap-2 rounded-lg bg-primary px-3.5 py-2 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
          type="button"
        >
          <Plus size={14} />
          连接新设备
        </button>
      </div>

      <div className="max-w-xl space-y-2">
        <p className="mb-3 font-mono text-[11px] uppercase tracking-widest text-muted-foreground">已配对设备</p>
        {devices.map((device) => (
          <div
            key={device.id}
            className="group flex items-center gap-4 rounded-xl border border-border bg-card p-4 transition-colors hover:border-primary/30"
          >
            <div className="rounded-lg bg-primary/10 p-2.5 text-primary">
              <OsIcon os={device.os} size={18} />
            </div>
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium text-foreground">{device.name}</p>
              <p className="font-mono text-[11px] text-muted-foreground">{device.ip}</p>
            </div>
            <div className="flex items-center gap-2">
              <StatusDot status={device.status} />
              <span className="text-xs text-muted-foreground">
                {device.status === "connected" ? "已连接" : device.status === "idle" ? "空闲" : "离线"}
              </span>
            </div>
            <button
              onClick={() => onRemoveDevice(device.id)}
              className="rounded-lg p-1.5 text-muted-foreground opacity-0 transition-all hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100"
              title="移除设备"
              type="button"
            >
              <X size={13} />
            </button>
          </div>
        ))}
        {devices.length === 0 && (
          <div className="py-12 text-center text-muted-foreground">
            <Smartphone size={32} className="mx-auto mb-3 opacity-30" />
            <p className="text-sm">暂无已配对设备</p>
            <p className="mt-1 text-xs">点击「连接新设备」开始配对</p>
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
  onSchemeChange,
  onThemeChange,
}: {
  colorScheme: ColorScheme;
  isDark: boolean;
  theme: ThemeColor;
  onSchemeChange: (scheme: ColorScheme) => void;
  onThemeChange: (theme: ThemeColor) => void;
}) {
  return (
    <div className="flex-1 max-w-3xl overflow-y-auto px-8 pt-8">
      <h2 className="mb-1 text-base font-semibold text-foreground">设置</h2>
      <p className="mb-6 text-sm text-muted-foreground">配置剪贴板同步行为与安全选项。</p>

      <div className="mb-6">
        <p className="mb-3 text-xs font-medium uppercase tracking-widest text-muted-foreground">外观</p>
        <div className="space-y-5 rounded-xl border border-border bg-card p-4">
          <div>
            <p className="mb-0.5 text-sm font-medium text-foreground">背景模式</p>
            <p className="mb-3 text-xs text-muted-foreground">设置界面的明暗风格</p>
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
            <p className="mb-0.5 text-sm font-medium text-foreground">主题色</p>
            <p className="mb-3 text-xs text-muted-foreground">切换界面强调色</p>
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
        </div>
      </div>

      <p className="mb-3 text-xs font-medium uppercase tracking-widest text-muted-foreground">同步</p>
      <div className="rounded-xl border border-border bg-card px-4">
        {[
          {
            label: "剪贴板变化时自动同步",
            desc: "当剪贴板有新内容时，立即推送至所有已连接设备",
            enabled: true,
          },
          {
            label: "同步图片",
            desc: "当前仅展示界面壳层，后续阶段再接入真实能力",
            enabled: false,
          },
          {
            label: "同步文件",
            desc: "当前仅展示界面壳层，后续阶段再接入真实能力",
            enabled: false,
          },
          {
            label: "加密传输内容",
            desc: "后续将与 Rust 后端的加密状态做真实联动",
            enabled: true,
          },
          {
            label: "显示接收通知",
            desc: "用于接收新内容时的桌面提示与悬浮反馈",
            enabled: true,
          },
        ].map((item) => (
          <div key={item.label} className="flex items-start justify-between gap-4 border-b border-border py-3.5 last:border-0">
            <div>
              <p className="text-sm font-medium text-foreground">{item.label}</p>
              <p className="mt-0.5 text-xs text-muted-foreground">{item.desc}</p>
            </div>
            <ToggleSwitch defaultOn={item.enabled} />
          </div>
        ))}
      </div>
    </div>
  );
}

function IncomingToast({ clip, device, onDismiss }: { clip: ClipEntry; device: Device; onDismiss: () => void }) {
  useEffect(() => {
    const timer = window.setTimeout(onDismiss, 4_000);
    return () => window.clearTimeout(timer);
  }, [onDismiss]);

  return (
    <div className="fixed bottom-5 right-5 z-50 w-72 rounded-xl border border-primary/40 bg-card p-4 shadow-2xl shadow-primary/10">
      <div className="flex items-start gap-3">
        <div className="mt-0.5 rounded bg-primary/10 p-1.5 text-primary">
          <Clipboard size={13} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="mb-0.5 text-xs font-medium text-foreground">收到新剪贴板内容</p>
          <p className="flex items-center gap-1 text-[11px] text-muted-foreground">
            <OsIcon os={device.os} size={10} />
            {device.name}
          </p>
          <p className="mt-1.5 truncate font-mono text-xs text-foreground/80">
            {clip.type === "text" ? clip.content.slice(0, 60) : clip.content}
          </p>
        </div>
        <button
          onClick={onDismiss}
          className="mt-0.5 text-lg leading-none text-muted-foreground hover:text-foreground"
          type="button"
        >
          ×
        </button>
      </div>
    </div>
  );
}

function PairingModal({
  onClose,
  onPaired,
}: {
  onClose: () => void;
  onPaired: (name: string, os: OS, ip: string) => void;
}) {
  const codeRef = useRef(String(Math.floor(100000 + Math.random() * 900000)));
  const [input, setInput] = useState("");
  const [step, setStep] = useState<"idle" | "pairing" | "success" | "error">("idle");
  const [pairingTarget, setPairingTarget] = useState<string | null>(null);

  const startPairing = useCallback(
    (code: string, targetName?: string, os?: OS, ip?: string) => {
      if (!/^\d{6}$/.test(code)) return;

      setPairingTarget(targetName ?? null);
      setStep("pairing");
      window.setTimeout(() => {
        if (code === "000000") {
          setStep("error");
          return;
        }

        setStep("success");
        window.setTimeout(() => {
          onPaired(targetName ?? `设备-${code}`, os ?? "windows", ip ?? "192.168.x.x:19876");
          onClose();
        }, 1_000);
      }, 1_500);
    },
    [onClose, onPaired],
  );

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
            className="rounded-lg p-1.5 text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            type="button"
          >
            <X size={15} />
          </button>
        </div>

        <div className="max-h-[80vh] space-y-5 overflow-y-auto p-5">
          <div className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3">
            {step === "idle" && (
              <>
                <WifiOff size={14} className="shrink-0 text-muted-foreground" />
                <span className="text-xs text-muted-foreground">未连接</span>
              </>
            )}
            {step === "pairing" && (
              <>
                <Loader2 size={14} className="shrink-0 animate-spin text-primary" />
                <span className="text-xs text-primary">正在配对{pairingTarget ? `「${pairingTarget}」` : ""}…</span>
              </>
            )}
            {step === "success" && (
              <>
                <ShieldCheck size={14} className="shrink-0 text-emerald-400" />
                <span className="text-xs text-emerald-400">配对成功，已建立加密连接</span>
              </>
            )}
            {step === "error" && (
              <>
                <WifiOff size={14} className="shrink-0 text-destructive" />
                <span className="text-xs text-destructive">配对码无效，请重试</span>
              </>
            )}
          </div>

          <div className="text-center">
            <p className="mb-3 font-mono text-[11px] uppercase tracking-widest text-muted-foreground">你的配对码</p>
            <div className="mb-3 flex items-center justify-center gap-2">
              {codeRef.current.split("").map((digit, index) => (
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
                onChange={(event) => setInput(event.target.value.replace(/\D/g, "").slice(0, 6))}
                disabled={step === "pairing" || step === "success"}
                className="flex-1 rounded-lg border border-border bg-secondary px-3 py-2.5 text-center font-mono text-lg tracking-[0.25em] text-foreground transition-colors placeholder:text-muted-foreground/50 focus:border-primary focus:outline-none disabled:opacity-50"
              />
              <button
                onClick={() => startPairing(input)}
                disabled={input.length !== 6 || step === "pairing" || step === "success"}
                className="rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                type="button"
              >
                {step === "pairing" ? <Loader2 size={15} className="animate-spin" /> : "配对"}
              </button>
            </div>
            {step === "error" && (
              <button
                onClick={() => {
                  setStep("idle");
                  setInput("");
                }}
                className="mt-2 text-xs text-primary hover:underline"
                type="button"
              >
                重新输入
              </button>
            )}
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
              {DISCOVERED_DEVICES.map((device) => (
                <div
                  key={device.ip}
                  className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3 transition-colors hover:border-primary/30"
                >
                  <div className="rounded-lg bg-primary/10 p-1.5 text-primary">
                    <OsIcon os={device.os} size={14} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="text-xs font-medium text-foreground">{device.name}</p>
                    <p className="font-mono text-[10px] text-muted-foreground">{device.ip}</p>
                  </div>
                  <button
                    onClick={() => startPairing(codeRef.current, device.name, device.os, device.ip)}
                    disabled={step === "pairing" || step === "success"}
                    className="shrink-0 rounded-lg bg-primary px-3 py-1.5 text-[11px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                    type="button"
                  >
                    连接设备
                  </button>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * 新前端壳层。
 * 当前阶段先迁入 React + Tailwind 结构，保留演示态数据；下一阶段再接回 Tauri 命令与事件。
 */
export default function App() {
  const [activeNav, setActiveNav] = useState<NavId>("clipboard");
  const [devices, setDevices] = useState<Device[]>(DEVICES);
  const [clips, setClips] = useState<ClipEntry[]>(INITIAL_CLIPS);
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [syncing, setSyncing] = useState(false);
  const [theme, setTheme] = useState<ThemeColor>(THEME_COLORS[0]);
  const [colorScheme, setColorScheme] = useState<ColorScheme>("dark");
  const [isDark, setIsDark] = useState(true);
  const [showPairing, setShowPairing] = useState(false);
  const [toast, setToast] = useState<{ clip: ClipEntry; device: Device } | null>(null);
  const localIp = "192.168.1.100";

  const syncTheme = useCallback(
    (nextTheme: ThemeColor, nextScheme: ColorScheme) => {
      applyColorScheme(nextScheme);
      applyThemeColor(nextTheme);
      setIsDark(isDarkActive());
    },
    [],
  );

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
    const timer = window.setTimeout(() => {
      const newClip: ClipEntry = {
        id: `live-${Date.now()}`,
        type: "text",
        content: "pnpm tauri dev",
        sourceDeviceId: "dev-2",
        size: "14 B",
        timestamp: new Date(),
      };

      setSyncing(true);
      window.setTimeout(() => {
        setClips((previous) => [newClip, ...previous]);
        setToast({ clip: newClip, device: DEVICES[1] });
        setSyncing(false);
      }, 900);
    }, 3_200);

    return () => window.clearTimeout(timer);
  }, []);

  return (
    <div className="flex h-screen w-full overflow-hidden bg-background text-foreground" style={{ fontFamily: "'Plus Jakarta Sans', sans-serif" }}>
      <Sidebar
        activeNav={activeNav}
        devices={devices}
        syncing={syncing}
        localIp={localIp}
        colorScheme={colorScheme}
        setColorScheme={setColorScheme}
        theme={theme}
        isDark={isDark}
        onThemeChange={setTheme}
        onNavigate={setActiveNav}
      />

      <main className="flex h-full min-w-0 flex-1 overflow-hidden">
        {activeNav === "clipboard" && (
          <>
            <ClipboardPage clips={clips} devices={devices} viewMode={viewMode} setViewMode={setViewMode} />
            <DevicesPanel devices={devices} />
          </>
        )}
        {activeNav === "devices" && (
          <DevicesPage
            devices={devices}
            onShowPairing={() => setShowPairing(true)}
            onRemoveDevice={(id) => setDevices((previous) => previous.filter((device) => device.id !== id))}
          />
        )}
        {activeNav === "settings" && (
          <SettingsPage
            colorScheme={colorScheme}
            isDark={isDark}
            theme={theme}
            onSchemeChange={setColorScheme}
            onThemeChange={setTheme}
          />
        )}
      </main>

      {showPairing && (
        <PairingModal
          onClose={() => setShowPairing(false)}
          onPaired={(name, os, ip) => {
            setDevices((previous) => [
              ...previous,
              {
                id: `dev-${Date.now()}`,
                name,
                os,
                ip,
                status: "connected",
                latency: Math.floor(5 + Math.random() * 20),
                lastSeen: new Date(),
              },
            ]);
          }}
        />
      )}

      {toast && <IncomingToast clip={toast.clip} device={toast.device} onDismiss={() => setToast(null)} />}
    </div>
  );
}
