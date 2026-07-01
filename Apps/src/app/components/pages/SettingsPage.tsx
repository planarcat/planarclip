import { Moon, Save, Sun, SunMoon } from "lucide-react";
import { useEffect, useState } from "react";
import { APP_DISPLAY_NAME } from "../../constants/app";
import { CLIPBOARD_HISTORY_LIMIT_OPTIONS } from "../../constants/clipboard";
import { MAX_MAX_FILE_MB, MIN_MAX_FILE_MB } from "../../constants/sync";
import { THEME_COLORS } from "../../constants/theme";
import type { ColorScheme, SettingAvailability, ThemeColor } from "../../types";
import { SettingBadge } from "../common/SettingBadge";
import { SettingToggle } from "../common/SettingToggle";
import { ThemeSwatch } from "../common/ThemeSwatch";

type SettingsPageProps = {
  colorScheme: ColorScheme;
  deviceName: string;
  isDark: boolean;
  theme: ThemeColor;
  isSaving: boolean;
  launchAtStartup: boolean;
  silentStart: boolean;
  isSavingStartupSettings: boolean;
  startupSettingsLoaded: boolean;
  autoConnectTrusted: boolean;
  isSavingConnectionSettings: boolean;
  connectionSettingsLoaded: boolean;
  onSchemeChange: (scheme: ColorScheme) => void;
  onThemeChange: (theme: ThemeColor) => void;
  onDeviceNameChange: (deviceName: string) => void;
  onDeviceNameSave: () => void;
  onLaunchAtStartupChange: (enabled: boolean) => void;
  onSilentStartChange: (enabled: boolean) => void;
  onAutoConnectTrustedChange: (enabled: boolean) => void;
  syncImages: boolean;
  syncFiles: boolean;
  maxFileMb: number;
  isSavingSyncSettings: boolean;
  syncSettingsLoaded: boolean;
  onSyncImagesChange: (enabled: boolean) => void;
  onSyncFilesChange: (enabled: boolean) => void;
  onMaxFileMbChange: (mb: number) => void;
  clipboardHistoryLimit: number;
  isSavingClipboardSettings: boolean;
  clipboardSettingsLoaded: boolean;
  onClipboardHistoryLimitChange: (limit: number) => void;
};

export function SettingsPage({
  colorScheme,
  deviceName,
  isDark,
  theme,
  isSaving,
  launchAtStartup,
  silentStart,
  isSavingStartupSettings,
  startupSettingsLoaded,
  autoConnectTrusted,
  isSavingConnectionSettings,
  connectionSettingsLoaded,
  onSchemeChange,
  onThemeChange,
  onDeviceNameChange,
  onDeviceNameSave,
  onLaunchAtStartupChange,
  onSilentStartChange,
  onAutoConnectTrustedChange,
  syncImages,
  syncFiles,
  maxFileMb,
  isSavingSyncSettings,
  syncSettingsLoaded,
  onSyncImagesChange,
  onSyncFilesChange,
  onMaxFileMbChange,
  clipboardHistoryLimit,
  isSavingClipboardSettings,
  clipboardSettingsLoaded,
  onClipboardHistoryLimitChange,
}: SettingsPageProps) {
  const [maxFileMbDraft, setMaxFileMbDraft] = useState(String(maxFileMb));

  useEffect(() => {
    setMaxFileMbDraft(String(maxFileMb));
  }, [maxFileMb]);

  const commitMaxFileMbDraft = () => {
    const nextValue = Number(maxFileMbDraft);
    if (
      !Number.isFinite(nextValue) ||
      !Number.isInteger(nextValue) ||
      nextValue < MIN_MAX_FILE_MB ||
      nextValue > MAX_MAX_FILE_MB
    ) {
      setMaxFileMbDraft(String(maxFileMb));
      return;
    }
    if (nextValue === maxFileMb) {
      return;
    }
    onMaxFileMbChange(nextValue);
  };

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
    <div className="flex-1 overflow-y-auto px-4 pt-6 md:px-6 md:pt-8 xl:px-8">
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

      <p className="mb-3 text-[13px] font-medium text-primary">启动与驻留</p>
      <div className="mb-6 rounded-xl border border-border bg-card px-4">
        <div className="flex items-start justify-between gap-4 border-b border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">登录时自动启动</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，系统登录时会自动启动{APP_DISPLAY_NAME}，并在后台继续监听剪贴板同步。
            </p>
          </div>
          <SettingToggle
            checked={launchAtStartup}
            disabled={!startupSettingsLoaded || isSavingStartupSettings}
            label="登录时自动启动"
            onChange={onLaunchAtStartupChange}
          />
        </div>
        <div className="flex items-start justify-between gap-4 py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">静默启动</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，启动时不显示主界面，只驻留托盘；关闭时，启动后会自动打开主界面。
            </p>
          </div>
          <SettingToggle
            checked={silentStart}
            disabled={!startupSettingsLoaded || isSavingStartupSettings}
            label="静默启动"
            onChange={onSilentStartChange}
          />
        </div>
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">连接</p>
      <div className="mb-6 rounded-xl border border-border bg-card px-4">
        <div className="flex items-start justify-between gap-4 py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">自动连接熟悉设备</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，应用启动时会尝试连接熟悉列表中的设备；局域网中发现熟悉设备上线时，也会自动发起连接。
            </p>
          </div>
          <SettingToggle
            checked={autoConnectTrusted}
            disabled={!connectionSettingsLoaded || isSavingConnectionSettings}
            label="自动连接熟悉设备"
            onChange={onAutoConnectTrustedChange}
          />
        </div>
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">剪贴板</p>
      <div className="mb-6 rounded-xl border border-border bg-card px-4">
        <div className="flex items-start justify-between gap-4 py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">展示记录上限</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              控制剪贴板历史页面最多保留多少条同步摘要，超出后会自动移除较早的记录。
            </p>
          </div>
          <select
            value={clipboardHistoryLimit}
            disabled={!clipboardSettingsLoaded || isSavingClipboardSettings}
            onChange={(event) => {
              onClipboardHistoryLimitChange(Number(event.target.value));
            }}
            aria-label="剪贴板展示记录上限"
            className="shrink-0 rounded-lg border border-border bg-secondary px-3 py-2 text-sm font-medium text-foreground transition-colors focus:border-primary focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
          >
            {CLIPBOARD_HISTORY_LIMIT_OPTIONS.map((limit) => (
              <option key={limit} value={limit}>
                {limit} 条
              </option>
            ))}
          </select>
        </div>
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">同步与安全</p>
      <div className="rounded-xl border border-border bg-card px-4">
        <div className="flex items-start justify-between gap-4 border-b border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">同步图片</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
            开启后，复制截图或图片会自动同步到已连接设备；单张图片最大 5 MB。
            </p>
          </div>
          <SettingToggle
            checked={syncImages}
            disabled={!syncSettingsLoaded || isSavingSyncSettings}
            label="同步图片"
            onChange={onSyncImagesChange}
          />
        </div>
        <div className="flex items-start justify-between gap-4 border-b border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">同步文件</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，在资源管理器中复制文件会自动同步到已连接设备。
            </p>
          </div>
          <SettingToggle
            checked={syncFiles}
            disabled={!syncSettingsLoaded || isSavingSyncSettings}
            label="同步文件"
            onChange={onSyncFilesChange}
          />
        </div>
        {syncFiles && (
          <div className="flex items-start justify-between gap-4 border-b border-border py-3.5">
            <div>
              <p className="text-sm font-medium text-primary">文件大小上限</p>
              <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
                单个文件超过此上限时不会同步。
              </p>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <input
                type="number"
                min={MIN_MAX_FILE_MB}
                max={MAX_MAX_FILE_MB}
                step={1}
                value={maxFileMbDraft}
                disabled={!syncSettingsLoaded || isSavingSyncSettings}
                aria-label="文件大小上限"
                onChange={(event) => {
                  setMaxFileMbDraft(event.target.value);
                }}
                onBlur={commitMaxFileMbDraft}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    commitMaxFileMbDraft();
                  }
                }}
                className="w-24 rounded-lg border border-border bg-secondary px-3 py-2 text-sm font-medium text-foreground transition-colors focus:border-primary focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
              />
              <span className="text-sm font-medium text-muted-foreground">MB</span>
            </div>
          </div>
        )}
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
