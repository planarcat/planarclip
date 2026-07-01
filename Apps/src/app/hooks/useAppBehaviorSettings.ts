import { useCallback, useEffect, useState } from "react";
import type { AppBehaviorSettingsPayload, CloseWindowAction, CommandExecutor } from "../types";
import { normalizeUserMessage } from "../utils/message";

const DEFAULT_APP_BEHAVIOR: AppBehaviorSettingsPayload = {
  system_notifications_enabled: true,
  close_window_action: "tray",
};

function normalizeCloseWindowAction(value: string): CloseWindowAction {
  return value === "exit" ? "exit" : "tray";
}

type UseAppBehaviorSettingsOptions = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  setLastMessage: (message: string) => void;
};

export function useAppBehaviorSettings({
  tauriAvailable,
  callCommand,
  setLastMessage,
}: UseAppBehaviorSettingsOptions) {
  const [systemNotificationsEnabled, setSystemNotificationsEnabled] = useState(
    DEFAULT_APP_BEHAVIOR.system_notifications_enabled,
  );
  const [closeWindowAction, setCloseWindowAction] = useState<CloseWindowAction>(
    DEFAULT_APP_BEHAVIOR.close_window_action,
  );
  const [isSaving, setIsSaving] = useState(false);
  const [loaded, setLoaded] = useState(!tauriAvailable);

  useEffect(() => {
    if (!tauriAvailable) {
      return;
    }

    let disposed = false;

    const load = async () => {
      try {
        const settings = await callCommand<AppBehaviorSettingsPayload>("get_app_behavior_settings");
        if (disposed) {
          return;
        }
        setSystemNotificationsEnabled(settings.system_notifications_enabled);
        setCloseWindowAction(normalizeCloseWindowAction(settings.close_window_action));
      } catch {
        if (!disposed) {
          setSystemNotificationsEnabled(DEFAULT_APP_BEHAVIOR.system_notifications_enabled);
          setCloseWindowAction(DEFAULT_APP_BEHAVIOR.close_window_action);
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

  const persistAppBehaviorSettings = useCallback(
    async (
      nextNotifications: boolean,
      nextCloseAction: CloseWindowAction,
      successMessage: string,
    ) => {
      if (!tauriAvailable) {
        setSystemNotificationsEnabled(nextNotifications);
        setCloseWindowAction(nextCloseAction);
        setLastMessage("当前是浏览器预览模式，通知与窗口设置仅用于界面预览。");
        return;
      }

      setIsSaving(true);
      try {
        const saved = await callCommand<AppBehaviorSettingsPayload>("save_app_behavior_settings", {
          systemNotificationsEnabled: nextNotifications,
          closeWindowAction: nextCloseAction,
        });
        setSystemNotificationsEnabled(saved.system_notifications_enabled);
        setCloseWindowAction(normalizeCloseWindowAction(saved.close_window_action));
        setLastMessage(successMessage);
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [callCommand, setLastMessage, tauriAvailable],
  );

  const handleSystemNotificationsChange = useCallback(
    (enabled: boolean) => {
      setSystemNotificationsEnabled(enabled);
      void persistAppBehaviorSettings(
        enabled,
        closeWindowAction,
        enabled ? "已开启系统通知。" : "已关闭系统通知，连接与同步事件不再弹出系统提醒。",
      );
    },
    [closeWindowAction, persistAppBehaviorSettings],
  );

  const handleCloseWindowActionChange = useCallback(
    (action: CloseWindowAction) => {
      if (action === closeWindowAction) {
        return;
      }
      setCloseWindowAction(action);
      void persistAppBehaviorSettings(
        systemNotificationsEnabled,
        action,
        action === "tray"
          ? "关闭窗口时将隐藏到托盘，应用继续在后台运行。"
          : "关闭窗口时将退出应用。",
      );
    },
    [closeWindowAction, persistAppBehaviorSettings, systemNotificationsEnabled],
  );

  return {
    systemNotificationsEnabled,
    closeWindowAction,
    isSavingAppBehaviorSettings: isSaving,
    appBehaviorSettingsLoaded: loaded,
    handleSystemNotificationsChange,
    handleCloseWindowActionChange,
  };
}
