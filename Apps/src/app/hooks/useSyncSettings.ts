import { useCallback, useEffect, useState } from "react";
import { DEFAULT_MAX_FILE_MB, MAX_MAX_FILE_MB, MIN_MAX_FILE_MB } from "../constants/sync";
import type { CommandExecutor, SyncSettingsPayload } from "../types";
import { normalizeUserMessage } from "../utils/message";

const DEFAULT_SYNC_SETTINGS: SyncSettingsPayload = {
  sync_images: true,
  sync_files: true,
  max_file_mb: DEFAULT_MAX_FILE_MB,
  auto_sync_clipboard: true,
  sync_files_save_enabled: false,
  sync_files_save_dir: "",
  sync_files_save_dir_is_default: true,
};

function isValidMaxFileMb(value: number): boolean {
  return Number.isInteger(value) && value >= MIN_MAX_FILE_MB && value <= MAX_MAX_FILE_MB;
}

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
  const [syncFiles, setSyncFiles] = useState(DEFAULT_SYNC_SETTINGS.sync_files);
  const [maxFileMb, setMaxFileMb] = useState(DEFAULT_SYNC_SETTINGS.max_file_mb);
  const [syncFilesSaveEnabled, setSyncFilesSaveEnabled] = useState(
    DEFAULT_SYNC_SETTINGS.sync_files_save_enabled,
  );
  const [autoSyncClipboard, setAutoSyncClipboard] = useState(DEFAULT_SYNC_SETTINGS.auto_sync_clipboard);
  const [syncFilesSaveDir, setSyncFilesSaveDir] = useState(DEFAULT_SYNC_SETTINGS.sync_files_save_dir);
  const [syncFilesSaveDirIsDefault, setSyncFilesSaveDirIsDefault] = useState(
    DEFAULT_SYNC_SETTINGS.sync_files_save_dir_is_default,
  );
  const [isSaving, setIsSaving] = useState(false);
  const [loaded, setLoaded] = useState(!tauriAvailable);

  const applySavedSettings = useCallback((settings: SyncSettingsPayload) => {
    setSyncImages(settings.sync_images);
    setSyncFiles(settings.sync_files);
    setMaxFileMb(settings.max_file_mb);
    setSyncFilesSaveEnabled(settings.sync_files_save_enabled);
    setAutoSyncClipboard(settings.auto_sync_clipboard);
    setSyncFilesSaveDir(settings.sync_files_save_dir);
    setSyncFilesSaveDirIsDefault(settings.sync_files_save_dir_is_default);
  }, []);

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
        applySavedSettings(settings);
      } catch {
        if (!disposed) {
          applySavedSettings(DEFAULT_SYNC_SETTINGS);
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
  }, [applySavedSettings, callCommand, tauriAvailable]);

  const handleSyncImagesChange = useCallback(
    async (enabled: boolean) => {
      if (!tauriAvailable) {
        setSyncImages(enabled);
        setLastMessage("当前是浏览器预览模式，同步设置仅用于界面预览。");
        return;
      }

      setSyncImages(enabled);
      setIsSaving(true);
      try {
        const saved = await callCommand<SyncSettingsPayload>("save_sync_settings", {
          syncImages: enabled,
          syncFiles,
          maxFileMb,
        });
        applySavedSettings(saved);
        setLastMessage(
          enabled ? "已开启图片同步，复制图片后会自动同步到已连接设备。" : "已关闭图片同步。",
        );
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [applySavedSettings, callCommand, maxFileMb, setLastMessage, syncFiles, tauriAvailable],
  );

  const handleSyncFilesChange = useCallback(
    async (enabled: boolean) => {
      if (!tauriAvailable) {
        setSyncFiles(enabled);
        setLastMessage("当前是浏览器预览模式，同步设置仅用于界面预览。");
        return;
      }

      setSyncFiles(enabled);
      setIsSaving(true);
      try {
        const saved = await callCommand<SyncSettingsPayload>("save_sync_settings", {
          syncImages,
          syncFiles: enabled,
          maxFileMb,
        });
        applySavedSettings(saved);
        setLastMessage(
          enabled
            ? "已开启文件同步，复制文件后会自动同步到已连接设备。"
            : "已关闭文件同步。",
        );
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [applySavedSettings, callCommand, maxFileMb, setLastMessage, syncImages, tauriAvailable],
  );

  const handleMaxFileMbChange = useCallback(
    async (mb: number) => {
      if (!isValidMaxFileMb(mb)) {
        setLastMessage("文件大小上限无效，请输入 1 到 500 之间的整数。");
        return;
      }

      if (!tauriAvailable) {
        setMaxFileMb(mb);
        setLastMessage("当前是浏览器预览模式，同步设置仅用于界面预览。");
        return;
      }

      if (mb === maxFileMb) {
        return;
      }

      setIsSaving(true);
      try {
        const saved = await callCommand<SyncSettingsPayload>("save_sync_settings", {
          syncImages,
          syncFiles,
          maxFileMb: mb,
        });
        applySavedSettings(saved);
        setLastMessage(`已更新文件大小上限为 ${saved.max_file_mb} MB。`);
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [applySavedSettings, callCommand, maxFileMb, setLastMessage, syncFiles, syncImages, tauriAvailable],
  );

  const handleAutoSyncClipboardChange = useCallback(
    async (enabled: boolean) => {
      if (!tauriAvailable) {
        setAutoSyncClipboard(enabled);
        setLastMessage("当前是浏览器预览模式，同步设置仅用于界面预览。");
        return;
      }

      setAutoSyncClipboard(enabled);
      setIsSaving(true);
      try {
        const saved = await callCommand<SyncSettingsPayload>("save_sync_settings", {
          syncImages,
          syncFiles,
          maxFileMb,
          autoSyncClipboard: enabled,
        });
        applySavedSettings(saved);
        setLastMessage(
          enabled
            ? "已开启自动同步，复制后会自动同步到已连接设备。"
            : "已关闭自动同步，可在剪贴板记录中点击发送手动同步。",
        );
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [applySavedSettings, callCommand, maxFileMb, setLastMessage, syncFiles, syncImages, tauriAvailable],
  );

  const handleSyncFilesSaveEnabledChange = useCallback(
    async (enabled: boolean) => {
      if (!tauriAvailable) {
        setSyncFilesSaveEnabled(enabled);
        setLastMessage("当前是浏览器预览模式，同步设置仅用于界面预览。");
        return;
      }

      setSyncFilesSaveEnabled(enabled);
      setIsSaving(true);
      try {
        const saved = await callCommand<SyncSettingsPayload>("save_sync_settings", {
          syncImages,
          syncFiles,
          maxFileMb,
          syncFilesSaveEnabled: enabled,
        });
        applySavedSettings(saved);
        setLastMessage(
          enabled
            ? "已开启本地保存，同步成功的文件会保存到你设置的路径。"
            : "已关闭本地保存，同步成功的文件只会写入剪贴板。",
        );
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [applySavedSettings, callCommand, maxFileMb, setLastMessage, syncFiles, syncImages, tauriAvailable],
  );

  const handlePickSyncFilesSaveDir = useCallback(async () => {
    if (!tauriAvailable) {
      setLastMessage("当前是浏览器预览模式，同步设置仅用于界面预览。");
      return;
    }

    setIsSaving(true);
    try {
      const picked = await callCommand<string>("pick_sync_files_save_dir");
      const saved = await callCommand<SyncSettingsPayload>("save_sync_files_save_dir", { path: picked });
      applySavedSettings(saved);
      setLastMessage("已更新同步文件保存路径。");
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
    } finally {
      setIsSaving(false);
    }
  }, [applySavedSettings, callCommand, setLastMessage, tauriAvailable]);

  const handleResetSyncFilesSaveDir = useCallback(async () => {
    if (!tauriAvailable) {
      setSyncFilesSaveDirIsDefault(true);
      setLastMessage("当前是浏览器预览模式，同步设置仅用于界面预览。");
      return;
    }

    setIsSaving(true);
    try {
      const saved = await callCommand<SyncSettingsPayload>("save_sync_files_save_dir", { path: null });
      applySavedSettings(saved);
      setLastMessage("已恢复为默认下载文件夹。");
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
    } finally {
      setIsSaving(false);
    }
  }, [applySavedSettings, callCommand, setLastMessage, tauriAvailable]);

  return {
    syncImages,
    syncFiles,
    maxFileMb,
    autoSyncClipboard,
    syncFilesSaveEnabled,
    syncFilesSaveDir,
    syncFilesSaveDirIsDefault,
    isSavingSyncSettings: isSaving,
    syncSettingsLoaded: loaded,
    handleSyncImagesChange,
    handleSyncFilesChange,
    handleAutoSyncClipboardChange,
    handleSyncFilesSaveEnabledChange,
    handleMaxFileMbChange,
    handlePickSyncFilesSaveDir,
    handleResetSyncFilesSaveDir,
  };
}
