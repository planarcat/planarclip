import { FolderOpen, Moon, RotateCcw, Sun, SunMoon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { APP_DISPLAY_NAME } from "../../constants/app";
import { CLIPBOARD_HISTORY_LIMIT_OPTIONS } from "../../constants/clipboard";
import { MAX_MAX_FILE_MB, MIN_MAX_FILE_MB } from "../../constants/sync";
import { THEME_COLORS } from "../../constants/theme";
import {
  SURFACE_REVEAL_BG,
  SURFACE_REVEAL_SELECT,
  SURFACE_REVEAL_TEXT_FIELD,
} from "../../constants/surfaceReveal";
import type { BroadcastState, ColorScheme, CloseWindowAction, CommandExecutor, SettingAvailability, ThemeColor } from "../../types";
import type { ThemePickOrigin } from "../../utils/themeTransition";
import { DiagnosticsPanel } from "../common/DiagnosticsPanel";
import { SettingBadge } from "../common/SettingBadge";
import { SettingToggleControl } from "../common/SettingToggle";
import { ThemeSwatch } from "../common/ThemeSwatch";
import { ScrollArea } from "../ui/ScrollArea";

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
  onThemeChange: (theme: ThemeColor, origin?: ThemePickOrigin) => void;
  appearanceLocked?: boolean;
  onDeviceNameChange: (deviceName: string) => void;
  onDeviceNameSave: () => void;
  onLaunchAtStartupChange: (enabled: boolean) => void;
  onSilentStartChange: (enabled: boolean) => void;
  systemNotificationsEnabled: boolean;
  closeWindowAction: CloseWindowAction;
  isSavingAppBehaviorSettings: boolean;
  appBehaviorSettingsLoaded: boolean;
  onSystemNotificationsChange: (enabled: boolean) => void;
  onCloseWindowActionChange: (action: CloseWindowAction) => void;
  onAutoConnectTrustedChange: (enabled: boolean) => void;
  syncImages: boolean;
  syncFiles: boolean;
  maxFileMb: number;
  isSavingSyncSettings: boolean;
  syncSettingsLoaded: boolean;
  onSyncImagesChange: (enabled: boolean) => void;
  autoSyncClipboard: boolean;
  onAutoSyncClipboardChange: (enabled: boolean) => void;
  onSyncFilesChange: (enabled: boolean) => void;
  syncFilesSaveEnabled: boolean;
  onSyncFilesSaveEnabledChange: (enabled: boolean) => void;
  onMaxFileMbChange: (mb: number) => void;
  syncFilesSaveDir: string;
  syncFilesSaveDirIsDefault: boolean;
  onPickSyncFilesSaveDir: () => void;
  onResetSyncFilesSaveDir: () => void;
  clipboardHistoryLimit: number;
  isSavingClipboardSettings: boolean;
  clipboardSettingsLoaded: boolean;
  onClipboardHistoryLimitChange: (limit: number) => void;
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  broadcastState?: BroadcastState;
  onPortChange: (port: number) => void;
};

export function SettingsPage({
  colorScheme,
  deviceName,
  isDark,
  theme,
  isSaving,
  launchAtStartup,
  silentStart,
  isSavingStartupSettings: _isSavingStartupSettings,
  startupSettingsLoaded,
  autoConnectTrusted,
  isSavingConnectionSettings: _isSavingConnectionSettings,
  connectionSettingsLoaded,
  onSchemeChange,
  onThemeChange,
  appearanceLocked = false,
  onDeviceNameChange,
  onDeviceNameSave,
  onLaunchAtStartupChange,
  onSilentStartChange,
  systemNotificationsEnabled,
  closeWindowAction,
  isSavingAppBehaviorSettings: _isSavingAppBehaviorSettings,
  appBehaviorSettingsLoaded,
  onSystemNotificationsChange,
  onCloseWindowActionChange,
  onAutoConnectTrustedChange,
  syncImages,
  syncFiles,
  maxFileMb,
  isSavingSyncSettings: _isSavingSyncSettings,
  syncSettingsLoaded,
  onSyncImagesChange,
  autoSyncClipboard,
  onAutoSyncClipboardChange,
  onSyncFilesChange,
  syncFilesSaveEnabled,
  onSyncFilesSaveEnabledChange,
  onMaxFileMbChange,
  syncFilesSaveDir,
  syncFilesSaveDirIsDefault,
  onPickSyncFilesSaveDir,
  onResetSyncFilesSaveDir,
  clipboardHistoryLimit,
  isSavingClipboardSettings,
  clipboardSettingsLoaded,
  onClipboardHistoryLimitChange,
  tauriAvailable,
  callCommand,
  broadcastState,
  onPortChange,
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

  // Auto-save device name (debounced) -- no save button needed.
  const onDeviceNameSaveRef = useRef(onDeviceNameSave);
  onDeviceNameSaveRef.current = onDeviceNameSave;
  const isFirstDeviceNameRender = useRef(true);
  useEffect(() => {
    if (isFirstDeviceNameRender.current) {
      isFirstDeviceNameRender.current = false;
      return;
    }
    const timer = setTimeout(() => onDeviceNameSaveRef.current(), 600);
    return () => clearTimeout(timer);
  }, [deviceName]);

  const settingRows: Array<{
    label: string;
    desc: string;
    availability: SettingAvailability;
  }> = [
    {
      label: "加密传输内容",
      desc: "当前直连链路默认启用加密传输，无需额外设置。",
      availability: "managed",
    },
  ];

  return (
    <ScrollArea className="flex-1 overflow-y-auto px-4 pb-8 pt-6 md:px-6 md:pb-10 md:pt-8 xl:px-8">
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
              className={`w-full ${SURFACE_REVEAL_TEXT_FIELD}`}
            />
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
                  disabled={appearanceLocked}
                  title={appearanceLocked ? "外观切换中，请稍候" : item.label}
                  className={`flex min-h-[4.5rem] flex-1 flex-col items-center justify-center gap-1.5 rounded-lg border py-3 ${SURFACE_REVEAL_BG} disabled:cursor-not-allowed disabled:opacity-40 ${
                    colorScheme === item.id
                      ? "border-primary bg-primary text-white hover:bg-[var(--button-primary-hover-bg)]"
                      : "border-border bg-transparent text-secondary-foreground hover:border-primary/40 hover:bg-secondary/40 hover:text-foreground"
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
                  disabled={appearanceLocked}
                />
              ))}
            </div>
          </div>
        </div>
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">启动与驻留</p>
      <div className="mb-6 rounded-xl border border-border bg-card px-4">
        <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">登录时自动启动</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，系统登录时会自动启动{APP_DISPLAY_NAME}，并在后台继续监听剪贴板同步。
            </p>
          </div>
          <SettingToggleControl
            checked={launchAtStartup}
            disabled={!startupSettingsLoaded}
            label="登录时自动启动"
            onChange={onLaunchAtStartupChange}
          />
        </div>
        <div className="flex items-center justify-between gap-4 py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">静默启动</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，启动时不显示主界面，只驻留托盘；关闭时，启动后会自动打开主界面。
            </p>
          </div>
          <SettingToggleControl
            checked={silentStart}
            disabled={!startupSettingsLoaded}
            label="静默启动"
            onChange={onSilentStartChange}
          />
        </div>
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">通知与窗口</p>
      <div className="mb-6 rounded-xl border border-border bg-card px-4">
        <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">系统通知</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，连接请求、同步失败等事件会通过系统通知提醒你；关闭后仅保留应用内提示。
            </p>
          </div>
          <SettingToggleControl
            checked={systemNotificationsEnabled}
            disabled={!appBehaviorSettingsLoaded}
            label="系统通知"
            onChange={onSystemNotificationsChange}
          />
        </div>
        <div className="py-3.5">
          <div className="mb-3">
            <p className="text-sm font-medium text-primary">点击关闭窗口时</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              选择点击窗口右上角关闭按钮后的行为。
            </p>
          </div>
          <div className="flex gap-2">
            {(
              [
                { id: "tray" as const, label: "隐藏到托盘" },
                { id: "exit" as const, label: "退出程序" },
              ] as const
            ).map((item) => (
              <button
                key={item.id}
                type="button"
                disabled={!appBehaviorSettingsLoaded}
                onClick={() => onCloseWindowActionChange(item.id)}
                className={`flex flex-1 items-center justify-center rounded-lg border px-3 py-2.5 text-sm font-medium ${SURFACE_REVEAL_BG} ${
                  closeWindowAction === item.id
                    ? "border-primary bg-primary text-white hover:bg-[var(--button-primary-hover-bg)]"
                    : "border-border bg-transparent text-secondary-foreground hover:border-primary/40 hover:bg-secondary/40 hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
                }`}
              >
                {item.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">连接</p>
      <div className="mb-6 rounded-xl border border-border bg-card px-4">
        <PortField
          broadcastState={broadcastState}
          onPortChange={onPortChange}
          disabled={!connectionSettingsLoaded}
        />
        <div className="flex items-center justify-between gap-4 border-t border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">自动连接熟悉设备</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，应用启动时会尝试连接熟悉列表中的设备；局域网中发现熟悉设备上线时，也会自动发起连接。
            </p>
          </div>
          <SettingToggleControl
            checked={autoConnectTrusted}
            disabled={!connectionSettingsLoaded}
            label="自动连接熟悉设备"
            onChange={onAutoConnectTrustedChange}
          />
        </div>
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">剪贴板</p>
      <div className="mb-6 rounded-xl border border-border bg-card px-4">
        <div className="flex items-center justify-between gap-4 py-3.5">
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
            className={`shrink-0 ${SURFACE_REVEAL_SELECT}`}
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
        <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">剪贴板变化时自动同步</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，复制文本、图片或文件会自动同步到已连接设备；关闭后不会自动同步，可在剪贴板记录中点击发送手动同步。
            </p>
          </div>
          <SettingToggleControl
            checked={autoSyncClipboard}
            disabled={!syncSettingsLoaded}
            label="剪贴板变化时自动同步"
            onChange={onAutoSyncClipboardChange}
          />
        </div>
        <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">同步图片</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
            开启后，复制截图或图片会自动同步到已连接设备；单张图片最大 5 MB。
            </p>
          </div>
          <SettingToggleControl
            checked={syncImages}
            disabled={!syncSettingsLoaded}
            label="同步图片"
            onChange={onSyncImagesChange}
          />
        </div>
        <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
          <div>
            <p className="text-sm font-medium text-primary">同步文件</p>
            <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
              开启后，在资源管理器中复制文件会自动同步到已连接设备。
            </p>
          </div>
          <SettingToggleControl
            checked={syncFiles}
            disabled={!syncSettingsLoaded}
            label="同步文件"
            onChange={onSyncFilesChange}
          />
        </div>
        {syncFiles && (
          <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
            <div>
              <p className="text-sm font-medium text-primary">保存同步文件到本地</p>
              <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
                开启后，接收到的文件或文件夹会额外保存到本地；关闭时仅写入剪贴板，便于直接粘贴使用。
              </p>
            </div>
            <SettingToggleControl
              checked={syncFilesSaveEnabled}
              disabled={!syncSettingsLoaded}
              label="保存同步文件到本地"
              onChange={onSyncFilesSaveEnabledChange}
            />
          </div>
        )}
        {syncFiles && syncFilesSaveEnabled && (
          <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium text-primary">文件保存路径</p>
              <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
                同步成功后，接收到的文件或文件夹会自动保存到此路径；未自定义时使用系统下载文件夹。
              </p>
              <p
                className="mt-2 truncate text-[13px] font-medium text-foreground"
                title={syncFilesSaveDir}
              >
                {syncFilesSaveDirIsDefault ? `默认：${syncFilesSaveDir}` : syncFilesSaveDir}
              </p>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <button
                type="button"
                onClick={onPickSyncFilesSaveDir}
                disabled={!syncSettingsLoaded}
                className={`flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-transparent text-secondary-foreground ${SURFACE_REVEAL_BG} hover:border-primary/40 hover:bg-secondary hover:text-foreground disabled:cursor-not-allowed disabled:opacity-60`}
                title="选择保存文件夹"
                aria-label="选择保存文件夹"
              >
                <FolderOpen size={16} />
              </button>
              {!syncFilesSaveDirIsDefault && (
                <button
                  type="button"
                  onClick={onResetSyncFilesSaveDir}
                  disabled={!syncSettingsLoaded}
                  className={`flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-transparent text-secondary-foreground ${SURFACE_REVEAL_BG} hover:border-primary/40 hover:bg-secondary hover:text-foreground disabled:cursor-not-allowed disabled:opacity-60`}
                  title="恢复为默认下载文件夹"
                  aria-label="恢复为默认下载文件夹"
                >
                  <RotateCcw size={16} />
                </button>
              )}
            </div>
          </div>
        )}
        {syncFiles && (
          <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
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
                disabled={!syncSettingsLoaded}
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
                className={`w-24 ${SURFACE_REVEAL_TEXT_FIELD} px-3 py-2`}
              />
              <span className="text-sm font-medium text-muted-foreground">MB</span>
            </div>
          </div>
        )}
        {settingRows.map((item) => (
          <div key={item.label} className="flex items-center justify-between gap-4 border-b border-border py-3.5 last:border-0">
            <div>
              <p className="text-sm font-medium text-primary">{item.label}</p>
              <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">{item.desc}</p>
            </div>
            <SettingBadge availability={item.availability} />
          </div>
        ))}
      </div>

      <p className="mb-3 text-[13px] font-medium text-primary">诊断与日志</p>
      <div className="mb-6 rounded-xl border border-border bg-card px-4">
        <DiagnosticsPanel tauriAvailable={tauriAvailable} callCommand={callCommand} />
      </div>
    </ScrollArea>
  );
}

function PortField({
  broadcastState,
  onPortChange,
  disabled,
}: {
  broadcastState?: BroadcastState;
  onPortChange: (port: number) => void;
  disabled: boolean;
}) {
  const currentPort = broadcastState?.port ?? 0;
  const [draft, setDraft] = useState(currentPort ? String(currentPort) : "");

  useEffect(() => {
    if (currentPort) {
      setDraft(String(currentPort));
    }
  }, [currentPort]);

  const commit = () => {
    const value = Number(draft);
    if (Number.isInteger(value) && value >= 1024 && value <= 65535 && value !== currentPort) {
      onPortChange(value);
    } else {
      setDraft(currentPort ? String(currentPort) : "");
    }
  };

  const conflict = broadcastState?.state === "PortConflict";

  return (
    <div className="flex items-center justify-between gap-4 py-3.5">
      <div>
        <p className="text-sm font-medium text-primary">监听端口</p>
        <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
          局域网发现与连接使用的端口；被占用时应用不会广播，但仍可查询其他设备。
        </p>
      </div>
      <div className="flex flex-col items-end gap-1">
        <input
          type="number"
          min={1024}
          max={65535}
          value={draft}
          disabled={disabled}
          aria-label="监听端口"
          onChange={(event) => setDraft(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commit();
            }
          }}
          className={`w-24 px-3 py-2 ${SURFACE_REVEAL_TEXT_FIELD} ${conflict ? "border-red-500" : ""}`}
        />
        {conflict && (
          <span className="text-[11px] font-medium text-red-500">端口 {currentPort} 被占用</span>
        )}
      </div>
    </div>
  );
}
