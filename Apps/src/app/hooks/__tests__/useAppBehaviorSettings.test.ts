import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useAppBehaviorSettings } from "../useAppBehaviorSettings";
import type { AppBehaviorSettingsPayload } from "../../types";

function makeMocks() {
  const setLastMessage = vi.fn();
  const callCommand = vi.fn();
  return { setLastMessage, callCommand };
}

describe("useAppBehaviorSettings", () => {
  it("浏览器预览模式：使用默认值，loaded 直接为 true", () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    expect(result.current.systemNotificationsEnabled).toBe(true);
    expect(result.current.closeWindowAction).toBe("tray");
    expect(result.current.appBehaviorSettingsLoaded).toBe(true);
    expect(result.current.isSavingAppBehaviorSettings).toBe(false);
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("桌面端加载成功：从 callCommand 获取设置", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockResolvedValue({
      system_notifications_enabled: false,
      close_window_action: "exit",
    });
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.appBehaviorSettingsLoaded).toBe(true);
    });
    expect(result.current.systemNotificationsEnabled).toBe(false);
    expect(result.current.closeWindowAction).toBe("exit");
  });

  it("桌面端加载失败：回退到默认值", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockRejectedValue(new Error("network error"));
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.appBehaviorSettingsLoaded).toBe(true);
    });
    expect(result.current.systemNotificationsEnabled).toBe(true);
    expect(result.current.closeWindowAction).toBe("tray");
  });

  it("桌面端加载成功后 disposed 不更新状态", async () => {
    let resolveInvoke: (v: AppBehaviorSettingsPayload) => void = () => {};
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockReturnValue(new Promise((r) => { resolveInvoke = r; }));
    const { unmount } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    unmount();
    resolveInvoke({ system_notifications_enabled: false, close_window_action: "exit" });
    await vi.waitFor(() => {
      expect(callCommand).toHaveBeenCalledWith("get_app_behavior_settings");
    });
  });

  it("浏览器预览模式：handleSystemNotificationsChange 仅更新本地状态", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    await act(async () => {
      result.current.handleSystemNotificationsChange(false);
    });

    expect(result.current.systemNotificationsEnabled).toBe(false);
    expect(setLastMessage).toHaveBeenCalledWith(
      "当前是浏览器预览模式，通知与窗口设置仅用于界面预览。",
    );
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("桌面端：handleSystemNotificationsChange 调用 save 并更新", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ system_notifications_enabled: false, close_window_action: "tray" })
      .mockResolvedValueOnce({ system_notifications_enabled: false, close_window_action: "tray" });
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.appBehaviorSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleSystemNotificationsChange(false);
    });

    expect(callCommand).toHaveBeenCalledWith("save_app_behavior_settings", {
      systemNotificationsEnabled: false,
      closeWindowAction: "tray",
    });
    expect(result.current.systemNotificationsEnabled).toBe(false);
    expect(setLastMessage).toHaveBeenCalledWith("已关闭系统通知，连接与同步事件不再弹出系统提醒。");
  });

  it("桌面端：handleSystemNotificationsChange 保存失败时显示错误提示", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ system_notifications_enabled: true, close_window_action: "tray" })
      .mockRejectedValueOnce(new Error("save failed"));
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.appBehaviorSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleSystemNotificationsChange(false);
    });

    expect(setLastMessage).toHaveBeenCalledWith("这次没有保存成功，请稍后再试。");
    expect(result.current.isSavingAppBehaviorSettings).toBe(false);
  });

  it("浏览器预览模式：handleCloseWindowActionChange 更新本地状态", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    await act(async () => {
      result.current.handleCloseWindowActionChange("exit");
    });

    expect(result.current.closeWindowAction).toBe("exit");
    expect(setLastMessage).toHaveBeenCalledWith("当前是浏览器预览模式，通知与窗口设置仅用于界面预览。");
  });

  it("桌面端：handleCloseWindowActionChange 切换为 exit", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ system_notifications_enabled: true, close_window_action: "tray" })
      .mockResolvedValueOnce({ system_notifications_enabled: true, close_window_action: "exit" });
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.appBehaviorSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleCloseWindowActionChange("exit");
    });

    expect(callCommand).toHaveBeenCalledWith("save_app_behavior_settings", {
      systemNotificationsEnabled: true,
      closeWindowAction: "exit",
    });
    expect(result.current.closeWindowAction).toBe("exit");
    expect(setLastMessage).toHaveBeenCalledWith("关闭窗口时将退出应用。");
  });

  it("handleCloseWindowActionChange 与当前值相同则跳过保存", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    await act(async () => {
      result.current.handleCloseWindowActionChange("tray");
    });

    expect(callCommand).not.toHaveBeenCalled();
    expect(setLastMessage).not.toHaveBeenCalled();
  });

  it("normalizeCloseWindowAction 收窄：非 'exit' 值回退为 'tray'", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockResolvedValue({
      system_notifications_enabled: true,
      close_window_action: "unknown_value",
    });
    const { result } = renderHook(() =>
      useAppBehaviorSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.appBehaviorSettingsLoaded).toBe(true);
    });
    expect(result.current.closeWindowAction).toBe("tray");
  });
});
