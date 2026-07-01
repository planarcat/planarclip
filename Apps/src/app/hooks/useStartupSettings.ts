import { useCallback, useEffect, useState } from "react";
import type { CommandExecutor, StartupSettingsPayload } from "../types";
import { normalizeUserMessage } from "../utils/message";

const DEFAULT_STARTUP_SETTINGS: StartupSettingsPayload = {
  launch_at_startup: false,
  silent_start: false,
};

type UseStartupSettingsOptions = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  setLastMessage: (message: string) => void;
};

export function useStartupSettings({
  tauriAvailable,
  callCommand,
  setLastMessage,
}: UseStartupSettingsOptions) {
  const [launchAtStartup, setLaunchAtStartup] = useState(DEFAULT_STARTUP_SETTINGS.launch_at_startup);
  const [silentStart, setSilentStart] = useState(DEFAULT_STARTUP_SETTINGS.silent_start);
  const [isSaving, setIsSaving] = useState(false);
  const [loaded, setLoaded] = useState(!tauriAvailable);

  useEffect(() => {
    if (!tauriAvailable) {
      return;
    }

    let disposed = false;

    const load = async () => {
      try {
        const settings = await callCommand<StartupSettingsPayload>("get_startup_settings");
        if (disposed) {
          return;
        }
        setLaunchAtStartup(settings.launch_at_startup);
        setSilentStart(settings.silent_start);
      } catch {
        if (!disposed) {
          setLaunchAtStartup(DEFAULT_STARTUP_SETTINGS.launch_at_startup);
          setSilentStart(DEFAULT_STARTUP_SETTINGS.silent_start);
        }
      } finally {
        if (!disposed) {
          setLoaded(true);
        }
      }
    };

    void load();

    return () => {
      disposed = true;
    };
  }, [callCommand, tauriAvailable]);

  const persistStartupSettings = useCallback(
    async (nextLaunchAtStartup: boolean, nextSilentStart: boolean, successMessage: string) => {
      if (!tauriAvailable) {
        setLaunchAtStartup(nextLaunchAtStartup);
        setSilentStart(nextSilentStart);
        setLastMessage("当前是浏览器预览模式，启动设置仅用于界面预览。");
        return;
      }

      setIsSaving(true);
      try {
        const saved = await callCommand<StartupSettingsPayload>("save_startup_settings", {
          launchAtStartup: nextLaunchAtStartup,
          silentStart: nextSilentStart,
        });
        setLaunchAtStartup(saved.launch_at_startup);
        setSilentStart(saved.silent_start);
        setLastMessage(successMessage);
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [callCommand, setLastMessage, tauriAvailable],
  );

  const handleLaunchAtStartupChange = useCallback(
    (enabled: boolean) => {
      setLaunchAtStartup(enabled);
      void persistStartupSettings(
        enabled,
        silentStart,
        enabled ? "已开启登录时自动启动。" : "已关闭登录时自动启动。",
      );
    },
    [persistStartupSettings, silentStart],
  );

  const handleSilentStartChange = useCallback(
    (enabled: boolean) => {
      setSilentStart(enabled);
      void persistStartupSettings(
        launchAtStartup,
        enabled,
        enabled ? "已开启静默启动，下次启动时将只驻留托盘。" : "已关闭静默启动，下次启动时会打开主界面。",
      );
    },
    [launchAtStartup, persistStartupSettings],
  );

  return {
    launchAtStartup,
    silentStart,
    isSavingStartupSettings: isSaving,
    startupSettingsLoaded: loaded,
    handleLaunchAtStartupChange,
    handleSilentStartChange,
  };
}
