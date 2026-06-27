import { useCallback, useEffect, useState } from "react";
import type { CommandExecutor, SyncSettingsPayload } from "../types";
import { normalizeUserMessage } from "../utils/message";

const DEFAULT_SYNC_SETTINGS: SyncSettingsPayload = {
  sync_images: true,
  sync_files: false,
};

type UseSyncSettingsOptions = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  setLastMessage: (message: string) => void;
};

export function useSyncSettings({
  tauriAvailable,
  callCommand,
  setLastMessage,
}: UseSyncSettingsOptions) {
  const [syncImages, setSyncImages] = useState(DEFAULT_SYNC_SETTINGS.sync_images);
  const [isSaving, setIsSaving] = useState(false);
  const [loaded, setLoaded] = useState(!tauriAvailable);

  useEffect(() => {
    if (!tauriAvailable) {
      return;
    }

    let disposed = false;

    const load = async () => {
      try {
        const settings = await callCommand<SyncSettingsPayload>("get_sync_settings");
        if (disposed) {
          return;
        }
        setSyncImages(settings.sync_images);
      } catch {
        if (!disposed) {
          setSyncImages(DEFAULT_SYNC_SETTINGS.sync_images);
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

  const handleSyncImagesChange = useCallback(
    async (enabled: boolean) => {
      if (!tauriAvailable) {
        setSyncImages(enabled);
        setLastMessage("当前是浏览器预览模式，同步设置仅用于界面预览。");
        return;
      }

      setIsSaving(true);
      try {
        const saved = await callCommand<SyncSettingsPayload>("save_sync_settings", {
          syncImages: enabled,
        });
        setSyncImages(saved.sync_images);
        setLastMessage(
          enabled ? "已开启图片同步，复制图片后会自动同步到已连接设备。" : "已关闭图片同步。",
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
    syncImages,
    isSavingSyncSettings: isSaving,
    syncSettingsLoaded: loaded,
    handleSyncImagesChange,
  };
}
