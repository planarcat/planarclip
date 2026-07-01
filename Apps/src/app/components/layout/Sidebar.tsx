import { Clipboard, Moon, Radio, Settings, Sun, SunMoon } from "lucide-react";
import { useEffect, useState } from "react";
import { THEME_COLORS } from "../../constants/theme";
import {
  SURFACE_REVEAL_BG,
  SURFACE_REVEAL_NAV_ITEM,
  SURFACE_REVEAL_TEXT_FIELD_SM,
} from "../../constants/surfaceReveal";
import type { ColorScheme, NavId, ThemeColor } from "../../types";
import type { ThemePickOrigin } from "../../utils/themeTransition";
import { ThemeSwatch } from "../common/ThemeSwatch";

type SidebarProps = {
  activeNav: NavId;
  identityLabel: string;
  connectedDeviceCount: number;
  onlineDeviceCount: number;
  colorScheme: ColorScheme;
  setColorScheme: (scheme: ColorScheme) => void;
  theme: ThemeColor;
  isDark: boolean;
  isSavingDeviceName: boolean;
  onThemeChange: (theme: ThemeColor, origin?: ThemePickOrigin) => void;
  onNavigate: (nav: NavId) => void;
  onDeviceNameChange: (deviceName: string) => void;
  onDeviceNameSave: (deviceName?: string) => void;
};

export function Sidebar({
  activeNav,
  identityLabel,
  connectedDeviceCount,
  onlineDeviceCount,
  colorScheme,
  setColorScheme,
  theme,
  isDark,
  isSavingDeviceName,
  onThemeChange,
  onNavigate,
  onDeviceNameChange,
  onDeviceNameSave,
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
    { id: "devices" as const, icon: <Radio size={15} /> },
    { id: "settings" as const, label: "设置", icon: <Settings size={15} /> },
  ];
  const hasConnectedDevices = connectedDeviceCount > 0;

  const commitDeviceName = () => {
    setIsEditingDeviceName(false);
    onDeviceNameSave(draftDeviceName);
  };

  return (
    <aside className="flex h-full w-60 shrink-0 flex-col border-r border-border bg-card xl:w-64">
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
                  className={`w-full ${SURFACE_REVEAL_TEXT_FIELD_SM}`}
                  placeholder="我的设备"
                />
              ) : (
                <button
                  onClick={() => setIsEditingDeviceName(true)}
                  aria-label="修改设备名称"
                  className={`block w-full rounded-md px-2.5 py-1.5 text-left text-[13px] font-medium text-primary ${SURFACE_REVEAL_BG} hover:bg-secondary/50`}
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
            className={`${SURFACE_REVEAL_NAV_ITEM} ${
              activeNav === item.id
                ? "bg-primary text-white hover:bg-[var(--button-primary-hover-bg)]"
                : "bg-transparent text-primary/85 hover:bg-secondary hover:text-primary"
            }`}
            type="button"
          >
            <span className="flex h-[15px] w-[15px] items-center justify-center">{item.icon}</span>
            <span className="min-w-0 text-left">
            {item.id === "devices" ? (
              <span className="flex min-w-0 items-center gap-2">
                <span>设备</span>
                <span className="flex items-center gap-1">
                  <span
                    className={`inline-flex h-5 min-w-5 items-center justify-center rounded-full px-1 text-[11px] font-medium tabular-nums ${
                      activeNav === "devices"
                        ? hasConnectedDevices
                          ? "bg-white/15 text-white"
                          : "bg-white/10 text-white/65"
                        : hasConnectedDevices
                          ? "bg-secondary text-primary"
                          : "bg-secondary text-muted-foreground"
                    }`}
                  >
                    {connectedDeviceCount}
                  </span>
                  <span
                    aria-hidden="true"
                    className={`text-[10px] ${activeNav === "devices" ? "text-white/35" : "text-muted-foreground/45"}`}
                  >
                    /
                  </span>
                  <span
                    className={`inline-flex h-5 min-w-5 items-center justify-center rounded-full px-1 text-[11px] font-medium tabular-nums ${
                      activeNav === "devices" ? "bg-white/10 text-white/70" : "bg-secondary text-muted-foreground"
                    }`}
                  >
                    {onlineDeviceCount}
                  </span>
                </span>
              </span>
            ) : (
              item.label
            )}
            </span>
          </button>
        ))}
      </nav>

      <div className="mt-auto space-y-3 border-t border-border px-3 pb-3 pt-3">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[13px] font-medium text-primary">背景</span>
          <div className="flex items-center rounded-md bg-secondary p-0.5">
            {[
              { id: "light" as const, label: "浅色", icon: <Sun size={14} /> },
              { id: "dark" as const, label: "深色", icon: <Moon size={14} /> },
              { id: "system" as const, label: "跟随系统", icon: <SunMoon size={14} /> },
            ].map((option) => (
              <button
                key={option.id}
                onClick={() => setColorScheme(option.id)}
                title={option.label}
                className={`flex h-6 w-6 items-center justify-center rounded ${SURFACE_REVEAL_BG} ${colorScheme === option.id ? "bg-card text-foreground shadow-sm" : "text-secondary-foreground hover:bg-secondary/70 hover:text-foreground"}`}
                type="button"
              >
                {option.icon}
              </button>
            ))}
          </div>
        </div>
        <div className="flex items-center justify-between gap-2">
          <span className="text-[13px] font-medium text-primary">主题</span>
          <div className="flex items-center gap-1">
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
    </aside>
  );
}
