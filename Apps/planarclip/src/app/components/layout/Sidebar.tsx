import { Clipboard, Moon, Palette, Radio, Settings, Sun, SunMoon } from "lucide-react";
import { THEME_COLORS } from "../../constants/theme";
import type { AppConnectionStatus, ColorScheme, Device, NavId, ThemeColor } from "../../types";
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
  onThemeChange: (theme: ThemeColor) => void;
  onNavigate: (nav: NavId) => void;
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
  onThemeChange,
  onNavigate,
  tauriAvailable,
}: SidebarProps) {
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

  return (
    <aside className="flex h-full w-52 shrink-0 flex-col border-r border-border bg-card xl:w-56">
      <div className="border-b border-border px-4 pb-4 pt-5">
        <div className="flex items-center gap-2.5">
          <div className="flex h-7 w-7 items-center justify-center rounded-md bg-primary/15">
            <Clipboard size={14} className="text-primary" />
          </div>
          <div>
            <p className="text-sm font-semibold leading-none tracking-tight text-foreground">PlanarClip</p>
            <p className="mt-0.5 text-[13px] font-medium text-muted-foreground">{identityLabel}</p>
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
                ? "bg-primary/10 text-primary"
                : "text-secondary-foreground hover:bg-secondary hover:text-foreground"
            }`}
            type="button"
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </nav>

      <div className="px-4 pb-2 pt-4">
        <p className="mb-2 text-[13px] font-medium text-muted-foreground">设备列表</p>
        {devices.length > 0 ? (
          <div className="space-y-1.5">
            {devices.map((device) => (
              <div key={device.id} className="flex items-center justify-between gap-2">
                <span className="truncate text-[13px] font-medium text-foreground">{device.name}</span>
                <StatusDot status={device.status} size="lg" />
              </div>
            ))}
          </div>
        ) : (
          <div className="rounded-lg border border-dashed border-border px-3 py-3 text-[13px] font-medium text-muted-foreground">
            暂时还没有发现附近设备
          </div>
        )}
      </div>

      <div className="mt-auto space-y-3 border-t border-border px-3 pb-3 pt-3">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[13px] font-medium text-muted-foreground">背景</span>
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
          <span className="text-[13px] font-medium text-secondary-foreground">{statusLabel}</span>
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
