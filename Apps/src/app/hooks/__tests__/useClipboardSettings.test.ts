import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useClipboardSettings } from "../useClipboardSettings";
import type { ClipEntry, ClipboardSettingsPayload } from "../../types";
import { DEFAULT_CLIPBOARD_HISTORY_LIMIT, DEFAULT_CLIPBOARD_VIEW_MODE } from "../../constants/clipboard";

function makeMocks() {
  const setLastMessage = vi.fn();
  const callCommand = vi.fn();
  const setClips = vi.fn();
  return { setLastMessage, callCommand, setClips };
}

describe("useClipboardSettings", () => {
  it("浏览器预览模式：使用默认值，loaded 直接为 true", () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: false, callCommand, setLastMessage, setClips }));
    expect(result.current.historyLimit).toBe(DEFAULT_CLIPBOARD_HISTORY_LIMIT);
    expect(result.current.viewMode).toBe(DEFAULT_CLIPBOARD_VIEW_MODE);
    expect(result.current.clipboardSettingsLoaded).toBe(true);
    expect(result.current.isSavingClipboardSettings).toBe(false);
    expect(result.current.isClearingClipboardHistory).toBe(false);
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("桌面端加载成功：从 callCommand 获取设置", async () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    callCommand.mockResolvedValue({ history_limit: 200, view_mode: "list" });
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: true, callCommand, setLastMessage, setClips }));
    await waitFor(() => { expect(result.current.clipboardSettingsLoaded).toBe(true); });
    expect(result.current.historyLimit).toBe(200);
    expect(result.current.viewMode).toBe("list");
  });

  it("桌面端加载失败：回退到默认值", async () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    callCommand.mockRejectedValue(new Error("network error"));
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: true, callCommand, setLastMessage, setClips }));
    await waitFor(() => { expect(result.current.clipboardSettingsLoaded).toBe(true); });
    expect(result.current.historyLimit).toBe(DEFAULT_CLIPBOARD_HISTORY_LIMIT);
    expect(result.current.viewMode).toBe(DEFAULT_CLIPBOARD_VIEW_MODE);
  });

  it("loadOnMount=false 时不加载", () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: true, callCommand, setLastMessage, setClips, loadOnMount: false }));
    expect(result.current.clipboardSettingsLoaded).toBe(false);
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("applyDesktopClipboardSettings 设置状态并标记 loaded", () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: true, callCommand, setLastMessage, setClips, loadOnMount: false }));
    act(() => { result.current.applyDesktopClipboardSettings({ history_limit: 500, view_mode: "list" }); });
    expect(result.current.historyLimit).toBe(500);
    expect(result.current.viewMode).toBe("list");
    expect(result.current.clipboardSettingsLoaded).toBe(true);
  });

  it("handleHistoryLimitChange：保存并更新", async () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    callCommand.mockResolvedValueOnce({ history_limit: 100, view_mode: "grid" }).mockResolvedValueOnce({ history_limit: 200, view_mode: "grid" });
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: true, callCommand, setLastMessage, setClips }));
    await waitFor(() => { expect(result.current.clipboardSettingsLoaded).toBe(true); });
    await act(async () => { await result.current.handleHistoryLimitChange(200); });
    expect(callCommand).toHaveBeenCalledWith("save_clipboard_settings", { historyLimit: 200 });
    expect(result.current.historyLimit).toBe(200);
    expect(setLastMessage).toHaveBeenCalledWith("已更新展示上限为 200 条。");
  });

  it("handleHistoryLimitChange 浏览器预览模式：仅本地更新", async () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: false, callCommand, setLastMessage, setClips }));
    await act(async () => { await result.current.handleHistoryLimitChange(200); });
    expect(result.current.historyLimit).toBe(200);
    expect(setLastMessage).toHaveBeenCalledWith("当前是浏览器预览模式，剪贴板设置仅用于界面预览。");
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("handleViewModeChange 桌面端：保存并更新", async () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    callCommand.mockResolvedValueOnce({ history_limit: 100, view_mode: "grid" }).mockResolvedValueOnce({ history_limit: 100, view_mode: "list" });
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: true, callCommand, setLastMessage, setClips }));
    await waitFor(() => { expect(result.current.clipboardSettingsLoaded).toBe(true); });
    await act(async () => { await result.current.handleViewModeChange("list"); });
    expect(callCommand).toHaveBeenCalledWith("save_clipboard_settings", { viewMode: "list" });
    expect(result.current.viewMode).toBe("list");
  });

  it("handleViewModeChange 浏览器预览模式：保存到 localStorage", async () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: false, callCommand, setLastMessage, setClips }));
    await act(async () => { await result.current.handleViewModeChange("list"); });
    expect(result.current.viewMode).toBe("list");
    expect(window.localStorage.getItem("planarclip_clipboard_view_mode")).toBe("list");
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("handleClearHistory 桌面端：清空历史", async () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    callCommand.mockResolvedValueOnce({ history_limit: 100, view_mode: "grid" }).mockResolvedValueOnce(undefined);
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: true, callCommand, setLastMessage, setClips }));
    await waitFor(() => { expect(result.current.clipboardSettingsLoaded).toBe(true); });
    await act(async () => { await result.current.handleClearHistory(); });
    expect(callCommand).toHaveBeenCalledWith("clear_clipboard_history");
    expect(setClips).toHaveBeenCalledWith([]);
    expect(setLastMessage).toHaveBeenCalledWith("已清空剪贴板历史。");
  });

  it("handleClearHistory 浏览器预览模式：仅清空本地状态", async () => {
    const { callCommand, setLastMessage, setClips } = makeMocks();
    const { result } = renderHook(() => useClipboardSettings({ tauriAvailable: false, callCommand, setLastMessage, setClips }));
    await act(async () => { await result.current.handleClearHistory(); });
    expect(setClips).toHaveBeenCalledWith([]);
    expect(setLastMessage).toHaveBeenCalledWith("当前是浏览器预览模式，清空操作仅影响界面预览。");
    expect(callCommand).not.toHaveBeenCalled();
  });
});
