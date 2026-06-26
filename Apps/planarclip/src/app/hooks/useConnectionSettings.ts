import { useCallback, useEffect, useState } from "react";
import type { CommandExecutor, ConnectionSettingsPayload } from "../types";
import { normalizeUserMessage } from "../utils/message";

const DEFAULT_CONNECTION_SETTINGS: ConnectionSettingsPayload = {
  auto_connect_trusted: false,
};

type UseConnectionSettingsOptions = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  setLastMessage: (message: string) => void;
};

export function useConnectionSettings({
  tauriAvailable,
  callCommand,
  setLastMessage,
}: UseConnectionSettingsOptions) {
  const [autoConnectTrusted, setAutoConnectTrusted] = useState(
    DEFAULT_CONNECTION_SETTINGS.auto_connect_trusted,
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
        const settings = await callCommand<ConnectionSettingsPayload>("get_connection_settings");
        if (disposed) {
          return;
        }
        setAutoConnectTrusted(settings.auto_connect_trusted);
      } catch {
        if (!disposed) {
          setAutoConnectTrusted(DEFAULT_CONNECTION_SETTINGS.auto_connect_trusted);
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

  const handleAutoConnectTrustedChange = useCallback(
    async (enabled: boolean) => {
      if (!tauriAvailable) {
        setAutoConnectTrusted(enabled);
        setLastMessage("当前是浏览器预览模式，连接设置仅用于界面预览。");
        return;
      }

      setIsSaving(true);
      try {
        const saved = await callCommand<ConnectionSettingsPayload>("save_connection_settings", {
          autoConnectTrusted: enabled,
        });
        setAutoConnectTrusted(saved.auto_connect_trusted);
        setLastMessage(
          enabled
            ? "已开启自动连接已信任设备，启动或发现设备上线时会自动发起连接。"
            : "已关闭自动连接已信任设备。",
        );
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [callCommand, setLastMessage, tauriAvailable],
  );

  return {
    autoConnectTrusted,
    isSavingConnectionSettings: isSaving,
    connectionSettingsLoaded: loaded,
    handleAutoConnectTrustedChange,
  };
}
