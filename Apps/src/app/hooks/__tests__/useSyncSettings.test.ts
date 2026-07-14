import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useSyncSettings } from "../useSyncSettings";
import type { SyncSettingsPayload } from "../../types";
import { DEFAULT_MAX_FILE_MB } from "../../constants/sync";

const DEFAULT_SYNC_SETTINGS: SyncSettingsPayload = {
  sync_images: true,
  sync_files: true,
  max_file_mb: DEFAULT_MAX_FILE_MB,
  auto_sync_clipboard: true,
  sync_files_save_enabled: false,
  sync_files_save_dir: "",
  sync_files_save_dir_is_default: true,
};

function makeMocks() {
  const setLastMessage = vi.fn();
  const callCommand = vi.fn();
  return { setLastMessage, callCommand };
}

describe("useSyncSettings", () => {
  it("浏览器预览模式：使用默认值，loaded 直接为 true", () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() => useSyncSettings({ tauriAvailable: false, callCommand, setLastMessage }));
    expect(result.current.syncImages).toBe(true);
    expect(result.current.syncFiles).toBe(true);
    expect(result.current.maxFileMb).toBe(DEFAULT_MAX_FILE_MB);
    expect(result.current.autoSyncClipboard).toBe(true);
    expect(result.current.syncFilesSaveEnabled).toBe(false);
    expect(result.current.syncFilesSaveDir).toBe("");
    expect(result.current.syncFilesSaveDirIsDefault).toBe(true);
    expect(result.current.syncSettingsLoaded).toBe(true);
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("桌面端加载成功：从 callCommand 获取设置", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockResolvedValue({ sync_images: false, sync_files: true, max_file_mb: 100, auto_sync_clipboard: false, sync_files_save_enabled: true, sync_files_save_dir: "D:\\Downloads", sync_files_save_dir_is_default: false });
    const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
    await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
    expect(result.current.syncImages).toBe(false);
    expect(result.current.syncFiles).toBe(true);
    expect(result.current.maxFileMb).toBe(100);
    expect(result.current.autoSyncClipboard).toBe(false);
    expect(result.current.syncFilesSaveEnabled).toBe(true);
    expect(result.current.syncFilesSaveDir).toBe("D:\\Downloads");
    expect(result.current.syncFilesSaveDirIsDefault).toBe(false);
  });

  it("桌面端加载失败：回退到默认值", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockRejectedValue(new Error("network error"));
    const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
    await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
    expect(result.current.syncImages).toBe(true);
    expect(result.current.syncFiles).toBe(true);
    expect(result.current.maxFileMb).toBe(DEFAULT_MAX_FILE_MB);
    expect(result.current.autoSyncClipboard).toBe(true);
  });

  describe("handleSyncImagesChange", () => {
    it("桌面端：保存并更新", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand.mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, sync_images: true }).mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, sync_images: false });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleSyncImagesChange(false); });
      expect(callCommand).toHaveBeenCalledWith("save_sync_settings", { syncImages: false, syncFiles: true, maxFileMb: DEFAULT_MAX_FILE_MB });
      expect(result.current.syncImages).toBe(false);
    });
  });

  describe("handleSyncFilesChange", () => {
    it("桌面端：保存并更新", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand.mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, sync_files: true }).mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, sync_files: false });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleSyncFilesChange(false); });
      expect(callCommand).toHaveBeenCalledWith("save_sync_settings", { syncImages: true, syncFiles: false, maxFileMb: DEFAULT_MAX_FILE_MB });
      expect(result.current.syncFiles).toBe(false);
    });
  });

  describe("handleMaxFileMbChange", () => {
    it("桌面端：保存并更新", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand.mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, max_file_mb: 100 }).mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, max_file_mb: 200 });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleMaxFileMbChange(200); });
      expect(callCommand).toHaveBeenCalledWith("save_sync_settings", { syncImages: true, syncFiles: true, maxFileMb: 200 });
      expect(result.current.maxFileMb).toBe(200);
    });

    it("无效值（<1）显示错误提示", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand.mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, max_file_mb: 100 });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleMaxFileMbChange(0); });
      expect(setLastMessage).toHaveBeenCalledWith("文件大小上限无效，请输入 1 到 500 之间的整数。");
      expect(callCommand).toHaveBeenCalledTimes(1); // only get_sync_settings, no save
    });

    it("无效值（>500）显示错误提示", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand.mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, max_file_mb: 100 });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleMaxFileMbChange(999); });
      expect(setLastMessage).toHaveBeenCalledWith("文件大小上限无效，请输入 1 到 500 之间的整数。");
    });

    it("与当前值相同则跳过保存", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand.mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, max_file_mb: 100 });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleMaxFileMbChange(DEFAULT_MAX_FILE_MB); });
      expect(callCommand).toHaveBeenCalledTimes(1); // only get_sync_settings, no save
    });
  });

  describe("handleAutoSyncClipboardChange", () => {
    it("桌面端：保存并更新", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand.mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, auto_sync_clipboard: true }).mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, auto_sync_clipboard: false });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleAutoSyncClipboardChange(false); });
      expect(callCommand).toHaveBeenCalledWith("save_sync_settings", { syncImages: true, syncFiles: true, maxFileMb: DEFAULT_MAX_FILE_MB, autoSyncClipboard: false });
    });
  });

  describe("handleSyncFilesSaveEnabledChange", () => {
    it("桌面端：保存并更新", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand.mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, sync_files_save_enabled: false }).mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, sync_files_save_enabled: true });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleSyncFilesSaveEnabledChange(true); });
      expect(callCommand).toHaveBeenCalledWith("save_sync_settings", { syncImages: true, syncFiles: true, maxFileMb: DEFAULT_MAX_FILE_MB, syncFilesSaveEnabled: true });
    });
  });

  describe("handlePickSyncFilesSaveDir", () => {
    it("桌面端：选择目录并保存", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand
        .mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS })
        .mockResolvedValueOnce("D:\\Custom")
        .mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, sync_files_save_dir: "D:\\Custom", sync_files_save_dir_is_default: false });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handlePickSyncFilesSaveDir(); });
      expect(callCommand).toHaveBeenCalledWith("pick_sync_files_save_dir");
      expect(callCommand).toHaveBeenCalledWith("save_sync_files_save_dir", { path: "D:\\Custom" });
      expect(result.current.syncFilesSaveDir).toBe("D:\\Custom");
      expect(result.current.syncFilesSaveDirIsDefault).toBe(false);
    });
  });

  describe("handleResetSyncFilesSaveDir", () => {
    it("桌面端：重置并保存", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      callCommand
        .mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS })
        .mockResolvedValueOnce({ ...DEFAULT_SYNC_SETTINGS, sync_files_save_dir: "", sync_files_save_dir_is_default: true });
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: true, callCommand, setLastMessage }));
      await waitFor(() => { expect(result.current.syncSettingsLoaded).toBe(true); });
      await act(async () => { result.current.handleResetSyncFilesSaveDir(); });
      expect(callCommand).toHaveBeenCalledWith("save_sync_files_save_dir", { path: null });
    });
  });

  describe("浏览器预览模式", () => {
    it("handleSyncImagesChange 仅更新本地状态", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: false, callCommand, setLastMessage }));
      await act(async () => { result.current.handleSyncImagesChange(false); });
      expect(result.current.syncImages).toBe(false);
      expect(setLastMessage).toHaveBeenCalledWith("当前是浏览器预览模式，同步设置仅用于界面预览。");
      expect(callCommand).not.toHaveBeenCalled();
    });

    it("handleSyncFilesChange 仅更新本地状态", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: false, callCommand, setLastMessage }));
      await act(async () => { result.current.handleSyncFilesChange(false); });
      expect(result.current.syncFiles).toBe(false);
    });

    it("handleMaxFileMbChange 仅更新本地状态", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: false, callCommand, setLastMessage }));
      await act(async () => { result.current.handleMaxFileMbChange(200); });
      expect(result.current.maxFileMb).toBe(200);
    });

    it("handleAutoSyncClipboardChange 仅更新本地状态", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: false, callCommand, setLastMessage }));
      await act(async () => { result.current.handleAutoSyncClipboardChange(false); });
      expect(result.current.autoSyncClipboard).toBe(false);
    });

    it("handleSyncFilesSaveEnabledChange 仅更新本地状态", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: false, callCommand, setLastMessage }));
      await act(async () => { result.current.handleSyncFilesSaveEnabledChange(true); });
      expect(result.current.syncFilesSaveEnabled).toBe(true);
    });

    it("handlePickSyncFilesSaveDir 显示预览提示", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: false, callCommand, setLastMessage }));
      await act(async () => { result.current.handlePickSyncFilesSaveDir(); });
      expect(setLastMessage).toHaveBeenCalledWith("当前是浏览器预览模式，同步设置仅用于界面预览。");
    });

    it("handleResetSyncFilesSaveDir 仅更新本地状态", async () => {
      const { callCommand, setLastMessage } = makeMocks();
      const { result } = renderHook(() => useSyncSettings({ tauriAvailable: false, callCommand, setLastMessage }));
      await act(async () => { result.current.handleResetSyncFilesSaveDir(); });
      expect(result.current.syncFilesSaveDirIsDefault).toBe(true);
    });
  });
});
