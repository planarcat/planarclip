import { Clipboard, Moon, Palette, PlugZap, Radio, RefreshCw, Settings, Sun, SunMoon, Unplug } from "lucide-react";
import { useEffect, useState } from "react";
import { THEME_COLORS } from "../../constants/theme";
import type { AppConnectionStatus, ColorScheme, Device, NavId, ThemeColor } from "../../types";
import { MAX_CONNECTIONS } from "../../constants/connection";
import { StatusDot } from "../common/StatusDot";
import { ThemeSwatch } from "../common/ThemeSwatch";

type SidebarProps = {
  activeNav: NavId;
  devices: Device[];
  status: AppConnectionStatus;
  identityLabel: string;
  colorScheme: ColorScheme;
  setColorScheme: (scheme: ColorScheme) => void;
  theme: ThemeColor;
  isDark: boolean;
  isSavingDeviceName: boolean;
  onThemeChange: (theme: ThemeColor) => void;
  onNavigate: (nav: NavId) => void;
  onRefreshDevices: () => void;
  onConnectDevice: (device: Device) => void;
  onDeviceNameChange: (deviceName: string) => void;
  onDeviceNameSave: (deviceName?: string) => void;
  onDisconnect: () => void;
  isRefreshingDevices: boolean;
  tauriAvailable: boolean;
};

export function Sidebar({
  activeNav,
  devices,
  status,
  identityLabel,
  colorScheme,
  setColorScheme,
  theme,
  isDark,
  isSavingDeviceName,
  onThemeChange,
  onNavigate,
  onRefreshDevices,
  onConnectDevice,
  onDeviceNameChange,
  onDeviceNameSave,
  onDisconnect,
  isRefreshingDevices,
  tauriAvailable,
}: SidebarProps) {
  const [isEditingDeviceName, setIsEditingDeviceName] = useState(false);
  const [draftDeviceName, setDraftDeviceName] = useState(identityLabel);

  useEffect(() => {
    if (!isEditingDeviceName) {
      setDraftDeviceName(identityLabel);
    }
  }, [identityLabel, isEditingDeviceName]);

  const navItems = [
    { id: "clipboard" as const, label: "剪贴板", icon: <Clipboard size={15} /> },
    { id: "devices" as const, label: "设备", icon: <Radio size={15} /> },
    { id: "settings" as const, label: "设置", icon: <Settings size={15} /> },
  ];

  const statusLabel =
    status === "connecting" ? "连接中…" : status === "online" ? "已连接" : tauriAvailable ? "监听中" : "预览模式";

  const statusClassName =
    status === "connecting"
      ? "bg-primary animate-pulse"
      : status === "online"
        ? "bg-emerald-400"
        : tauriAvailable
          ? "bg-amber-400"
          : "bg-zinc-500";

  const commitDeviceName = () => {
    setIsEditingDeviceName(false);
    onDeviceNameSave(draftDeviceName);
  };

  return (
    <aside className="flex h-full w-52 shrink-0 flex-col border-r border-border bg-card xl:w-56">
      <div className="border-b border-border px-4 pb-4 pt-5">
        <div className="flex items-start gap-2.5">
          <div className="mt-0.5 flex h-7 w-7 items-center justify-center rounded-md bg-primary/15">
            <Clipboard size={14} className="text-primary" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold leading-none tracking-tight text-primary">本机名</p>
            <div className="mt-2">
              {isEditingDeviceName ? (
                <input
                  type="text"
                  value={draftDeviceName}
                  maxLength={24}
                  autoComplete="off"
                  autoFocus
                  disabled={isSavingDeviceName}
                  onChange={(event) => {
                    const nextDeviceName = event.target.value.slice(0, 24);
                    setDraftDeviceName(nextDeviceName);
                    onDeviceNameChange(nextDeviceName);
                  }}
                  onBlur={commitDeviceName}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      commitDeviceName();
                    }

                    if (event.key === "Escape") {
                      event.preventDefault();
                      setDraftDeviceName(identityLabel);
                      onDeviceNameChange(identityLabel);
                      setIsEditingDeviceName(false);
                    }
                  }}
                  className="w-full rounded-md border border-border bg-secondary px-2.5 py-1.5 text-[13px] font-medium text-primary transition-colors placeholder:text-primary/45 focus:border-primary focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
                  placeholder="我的设备"
                />
              ) : (
                <button
                  onClick={() => setIsEditingDeviceName(true)}
                  aria-label="修改设备名称"
                  className="block w-full bg-transparent p-0 text-left text-[13px] font-medium text-primary transition-opacity hover:opacity-80"
                  title="点击修改设备名称"
                  type="button"
                >
                  {identityLabel}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>

      <nav className="space-y-0.5 px-2 pb-2 pt-3">
        {navItems.map((item) => (
          <button
            key={item.id}
            onClick={() => onNavigate(item.id)}
            className={`flex w-full items-center gap-2.5 rounded px-2.5 py-2 text-sm font-medium transition-colors ${
              activeNav === item.id
                ? "bg-primary text-white"
                : "text-primary/85 hover:bg-secondary hover:text-primary"
            }`}
            type="button"
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </nav>

      <div className="px-4 pb-2 pt-4">
        <div className="mb-2 flex items-center justify-between gap-2">
          <p className="text-[13px] font-medium text-primary">设备列表</p>
          <button
            onClick={onRefreshDevices}
            disabled={isRefreshingDevices}
            aria-label="刷新设备列表"
            className="rounded-md p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-foreground disabled:opacity-40 cursor-pointer"
            title="刷新设备列表"
            type="button"
          >
            <RefreshCw size={13} className={isRefreshingDevices ? "animate-spin" : undefined} />
          </button>
        </div>
        {devices.length > 0 ? (
          <div className="space-y-1.5">
            {devices.map((device) => {
              const connectedCount = devices.filter((entry) => entry.status === "connected").length;
              const atConnectionLimit = connectedCount >= MAX_CONNECTIONS;
              const connectDisabled =
                status === "connecting" || (atConnectionLimit && device.status !== "connected");
              const hostNameLabel = device.hostName?.trim();
              const showHostName = Boolean(hostNameLabel && hostNameLabel.toLocaleLowerCase() !== device.name.trim().toLocaleLowerCase());
              const actionTitle =
                device.status === "connected"
                  ? `断开与 ${device.name} 的连接`
                  : status === "connecting"
                    ? `正在处理 ${device.name} 的连接`
                    : atConnectionLimit
                      ? "已超出连接上限，请先断开其中一个设备"
                      : `连接到 ${device.name}`;

              return (
                <div key={device.id} className="flex items-center justify-between gap-2 rounded-lg px-1 py-1 hover:bg-secondary/40">
                  <div className="min-w-0 flex-1" title={showHostName ? `${device.name} · ${hostNameLabel}` : device.name}>
                    <p className="truncate text-[13px] font-medium text-primary">{device.name}</p>
                    {showHostName && <p className="truncate text-[11px] font-medium text-primary/65">{hostNameLabel}</p>}
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    <StatusDot status={device.status} size="md" />
                    {device.status === "connected" ? (
                      <button
                        onClick={onDisconnect}
                        aria-label={actionTitle}
                        className="rounded-md p-1.5 text-secondary-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
                        title={actionTitle}
                        type="button"
                      >
                        <Unplug size={13} />
                      </button>
                    ) : (
                      <button
                        onClick={() => onConnectDevice(device)}
                        disabled={connectDisabled}
                        aria-label={actionTitle}
                        className="rounded-md p-1.5 text-secondary-foreground transition-colors hover:bg-primary/10 hover:text-primary disabled:opacity-40"
                        title={actionTitle}
                        type="button"
                      >
                        <PlugZap size={13} />
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="rounded-lg border border-dashed border-border px-3 py-3 text-center text-[13px] font-medium text-primary/75">
            暂无发现更多设备
          </div>
        )}
      </div>

      <div className="mt-auto space-y-3 border-t border-border px-3 pb-3 pt-3">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[13px] font-medium text-primary">背景</span>
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
                className={`rounded p-1.5 transition-colors ${colorScheme === option.id ? "bg-card text-foreground shadow-sm" : "text-secondary-foreground hover:text-foreground"}`}
                type="button"
              >
                {option.icon}
              </button>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <div className={`h-1.5 w-1.5 shrink-0 rounded-full ${statusClassName}`} />
          <span className="text-[13px] font-medium text-primary/80">{statusLabel}</span>
          <div className="ml-auto flex items-center gap-2">
            <Palette size={12} className="text-secondary-foreground" />
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
