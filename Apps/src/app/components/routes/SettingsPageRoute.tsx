import { SettingsPage } from "../pages/SettingsPage";
import { useAppBehaviorSettings } from "../../hooks/useAppBehaviorSettings";
import { useClipboardSettings } from "../../hooks/useClipboardSettings";
import { useConnectionSettings } from "../../hooks/useConnectionSettings";
import { useStartupSettings } from "../../hooks/useStartupSettings";
import { useSyncSettings } from "../../hooks/useSyncSettings";
import type { ColorScheme, CommandExecutor, ThemeColor } from "../../types";
import type { ThemePickOrigin } from "../../utils/themeTransition";

export type SettingsPageRouteProps = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  setLastMessage: (message: string) => void;
  colorScheme: ColorScheme;
  deviceName: string;
  isDark: boolean;
  theme: ThemeColor;
  isSavingDeviceName: boolean;
  appearanceLocked: boolean;
  onSchemeChange: (scheme: ColorScheme) => void;
  onThemeChange: (theme: ThemeColor, origin?: ThemePickOrigin) => void;
  onDeviceNameChange: (deviceName: string) => void;
  onDeviceNameSave: () => void;
};

export function SettingsPageRoute({
  tauriAvailable,
  callCommand,
  setLastMessage,
  colorScheme,
  deviceName,
  isDark,
  theme,
  isSavingDeviceName,
  appearanceLocked,
  onSchemeChange,
  onThemeChange,
  onDeviceNameChange,
  onDeviceNameSave,
}: SettingsPageRouteProps) {
  const {
    launchAtStartup,
    silentStart,
    isSavingStartupSettings,
    startupSettingsLoaded,
    handleLaunchAtStartupChange,
    handleSilentStartChange,
  } = useStartupSettings({
    tauriAvailable,
    callCommand,
    setLastMessage,
  });

  const {
    systemNotificationsEnabled,
    closeWindowAction,
    isSavingAppBehaviorSettings,
    appBehaviorSettingsLoaded,
    handleSystemNotificationsChange,
    handleCloseWindowActionChange,
  } = useAppBehaviorSettings({
    tauriAvailable,
    callCommand,
    setLastMessage,
  });

  const {
    autoConnectTrusted,
    isSavingConnectionSettings,
    connectionSettingsLoaded,
    handleAutoConnectTrustedChange,
  } = useConnectionSettings({
    tauriAvailable,
    callCommand,
    setLastMessage,
  });

  const {
    syncImages,
    syncFiles,
    maxFileMb,
    syncFilesSaveEnabled,
    syncFilesSaveDir,
    syncFilesSaveDirIsDefault,
    isSavingSyncSettings,
    syncSettingsLoaded,
    handleSyncImagesChange,
    handleSyncFilesChange,
    handleSyncFilesSaveEnabledChange,
    handleMaxFileMbChange,
    handlePickSyncFilesSaveDir,
    handleResetSyncFilesSaveDir,
    autoSyncClipboard,
    handleAutoSyncClipboardChange,
  } = useSyncSettings({
    tauriAvailable,
    callCommand,
    setLastMessage,
  });

  const {
    historyLimit,
    isSavingClipboardSettings,
    clipboardSettingsLoaded,
    handleHistoryLimitChange,
  } = useClipboardSettings({
    tauriAvailable,
    callCommand,
    setLastMessage,
    setClips: () => {},
    loadOnMount: true,
  });

  return (
    <SettingsPage
      colorScheme={colorScheme}
      deviceName={deviceName}
      isDark={isDark}
      theme={theme}
      isSaving={isSavingDeviceName}
      launchAtStartup={launchAtStartup}
      silentStart={silentStart}
      isSavingStartupSettings={isSavingStartupSettings}
      startupSettingsLoaded={startupSettingsLoaded}
      onSchemeChange={onSchemeChange}
      onThemeChange={onThemeChange}
      appearanceLocked={appearanceLocked}
      onDeviceNameChange={onDeviceNameChange}
      onDeviceNameSave={onDeviceNameSave}
      onLaunchAtStartupChange={handleLaunchAtStartupChange}
      onSilentStartChange={handleSilentStartChange}
      systemNotificationsEnabled={systemNotificationsEnabled}
      closeWindowAction={closeWindowAction}
      isSavingAppBehaviorSettings={isSavingAppBehaviorSettings}
      appBehaviorSettingsLoaded={appBehaviorSettingsLoaded}
      onSystemNotificationsChange={handleSystemNotificationsChange}
      onCloseWindowActionChange={handleCloseWindowActionChange}
      autoConnectTrusted={autoConnectTrusted}
      isSavingConnectionSettings={isSavingConnectionSettings}
      connectionSettingsLoaded={connectionSettingsLoaded}
      onAutoConnectTrustedChange={handleAutoConnectTrustedChange}
      syncImages={syncImages}
      syncFiles={syncFiles}
      maxFileMb={maxFileMb}
      isSavingSyncSettings={isSavingSyncSettings}
      syncSettingsLoaded={syncSettingsLoaded}
      onSyncImagesChange={handleSyncImagesChange}
      autoSyncClipboard={autoSyncClipboard}
      onAutoSyncClipboardChange={handleAutoSyncClipboardChange}
      onSyncFilesChange={handleSyncFilesChange}
      syncFilesSaveEnabled={syncFilesSaveEnabled}
      onSyncFilesSaveEnabledChange={handleSyncFilesSaveEnabledChange}
      onMaxFileMbChange={(mb) => {
        void handleMaxFileMbChange(mb);
      }}
      syncFilesSaveDir={syncFilesSaveDir}
      syncFilesSaveDirIsDefault={syncFilesSaveDirIsDefault}
      onPickSyncFilesSaveDir={() => {
        void handlePickSyncFilesSaveDir();
      }}
      onResetSyncFilesSaveDir={() => {
        void handleResetSyncFilesSaveDir();
      }}
      clipboardHistoryLimit={historyLimit}
      isSavingClipboardSettings={isSavingClipboardSettings}
      clipboardSettingsLoaded={clipboardSettingsLoaded}
      onClipboardHistoryLimitChange={(limit) => {
        void handleHistoryLimitChange(limit);
      }}
    />
  );
}
