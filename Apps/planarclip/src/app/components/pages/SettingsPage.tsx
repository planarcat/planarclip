import { Moon, Save, Sun, SunMoon } from "lucide-react";
import { THEME_COLORS } from "../../constants/theme";
import type { ColorScheme, SettingAvailability, ThemeColor } from "../../types";
import { SettingBadge } from "../common/SettingBadge";
import { ThemeSwatch } from "../common/ThemeSwatch";

type SettingsPageProps = {
  colorScheme: ColorScheme;
  deviceName: string;
  isDark: boolean;
  theme: ThemeColor;
  isSaving: boolean;
  onSchemeChange: (scheme: ColorScheme) => void;
  onThemeChange: (theme: ThemeColor) => void;
  onDeviceNameChange: (deviceName: string) => void;
  onDeviceNameSave: () => void;
};

export function SettingsPage({
  colorScheme,
  deviceName,
  isDark,
  theme,
  isSaving,
  onSchemeChange,
  onThemeChange,
  onDeviceNameChange,
  onDeviceNameSave,
}: SettingsPageProps) {
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
      <h2 className="mb-1 text-base font-semibold text-primary">设置</h2>
      <p className="mb-6 text-sm text-secondary-foreground">管理已落地的外观项、设备名称，并查看当前版本可用的同步能力边界。</p>

      <div className="mb-6">
        <p className="mb-3 text-[13px] font-medium text-primary">外观与设备</p>
        <div className="space-y-5 rounded-xl border border-border bg-card p-4">
          <div>
            <div className="mb-3 flex items-start justify-between gap-4">
              <div>
                <p className="mb-0.5 text-sm font-medium text-primary">设备名称</p>
                <p className="text-[13px] font-medium text-muted-foreground">其他设备会在设备列表与连接请求中看到这个名称。</p>
              </div>
              <SettingBadge availability="editable" />
            </div>
            <div className="flex gap-2">
              <input
                type="text"
                value={deviceName}
                maxLength={24}
                placeholder="我的设备"
                autoComplete="off"
                disabled={isSaving}
                onChange={(event) => onDeviceNameChange(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    onDeviceNameSave();
                  }
                }}
                className="flex-1 rounded-lg border border-border bg-secondary px-3 py-2.5 text-sm font-medium text-foreground transition-colors placeholder:text-muted-foreground/60 focus:border-primary focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
              />
              <button
                onClick={onDeviceNameSave}
                disabled={isSaving}
                className="flex shrink-0 items-center gap-2 rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
                title={isSaving ? "正在保存" : "保存设备名称"}
                type="button"
              >
                <Save size={14} />
              </button>
            </div>
          </div>

          <div>
            <div className="mb-3 flex items-start justify-between gap-4">
              <div>
                <p className="mb-0.5 text-sm font-medium text-primary">背景模式</p>
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
                      ? "border-primary bg-primary text-white"
                      : "border-border text-secondary-foreground hover:border-primary/40 hover:text-foreground"
                  }`}
                  type="button"
                >
                  {item.icon}
                  <span className="text-xs font-medium">{item.label}</span>
                </button>
              ))}
            </div>
          </div>

          <div>
            <div className="mb-3 flex items-start justify-between gap-4">
              <div>
                <p className="mb-0.5 text-sm font-medium text-primary">主题色</p>
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
                  size="lg"
                  onChange={onThemeChange}
                />
              ))}
            </div>
          </div>
        </div>
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">同步与安全</p>
      <div className="rounded-xl border border-border bg-card px-4">
        {settingRows.map((item) => (
          <div key={item.label} className="flex items-start justify-between gap-4 border-b border-border py-3.5 last:border-0">
            <div>
              <p className="text-sm font-medium text-primary">{item.label}</p>
              <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">{item.desc}</p>
            </div>
            <SettingBadge availability={item.availability} />
          </div>
        ))}
      </div>
    </div>
  );
}
