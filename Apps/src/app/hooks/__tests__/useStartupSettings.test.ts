import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useStartupSettings } from "../useStartupSettings";
import type { StartupSettingsPayload } from "../../types";

function makeMocks() {
  const setLastMessage = vi.fn();
  const callCommand = vi.fn();
  return { setLastMessage, callCommand };
}

describe("useStartupSettings", () => {
  it("浏览器预览模式：使用默认值，loaded 直接为 true", () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    expect(result.current.launchAtStartup).toBe(false);
    expect(result.current.silentStart).toBe(false);
    expect(result.current.startupSettingsLoaded).toBe(true);
    expect(result.current.isSavingStartupSettings).toBe(false);
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("桌面端加载成功：从 callCommand 获取设置", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockResolvedValue({
      launch_at_startup: true,
      silent_start: true,
    });
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.startupSettingsLoaded).toBe(true);
    });
    expect(result.current.launchAtStartup).toBe(true);
    expect(result.current.silentStart).toBe(true);
  });

  it("桌面端加载失败：回退到默认值", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockRejectedValue(new Error("network error"));
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.startupSettingsLoaded).toBe(true);
    });
    expect(result.current.launchAtStartup).toBe(false);
    expect(result.current.silentStart).toBe(false);
  });

  it("浏览器预览模式：handleLaunchAtStartupChange 仅更新本地状态", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    await act(async () => {
      result.current.handleLaunchAtStartupChange(true);
    });

    expect(result.current.launchAtStartup).toBe(true);
    expect(setLastMessage).toHaveBeenCalledWith("当前是浏览器预览模式，启动设置仅用于界面预览。");
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("桌面端：handleLaunchAtStartupChange 调用 save 并更新", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ launch_at_startup: false, silent_start: false })
      .mockResolvedValueOnce({ launch_at_startup: true, silent_start: false });
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.startupSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleLaunchAtStartupChange(true);
    });

    expect(callCommand).toHaveBeenCalledWith("save_startup_settings", {
      launchAtStartup: true,
      silentStart: false,
    });
    expect(result.current.launchAtStartup).toBe(true);
    expect(setLastMessage).toHaveBeenCalledWith("已开启登录时自动启动。");
  });

  it("桌面端：handleLaunchAtStartupChange 保存失败时显示错误", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ launch_at_startup: false, silent_start: false })
      .mockRejectedValueOnce(new Error("save failed"));
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.startupSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleLaunchAtStartupChange(true);
    });

    expect(setLastMessage).toHaveBeenCalledWith("这次没有保存成功，请稍后再试。");
    expect(result.current.isSavingStartupSettings).toBe(false);
  });

  it("浏览器预览模式：handleSilentStartChange 仅更新本地状态", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    await act(async () => {
      result.current.handleSilentStartChange(true);
    });

    expect(result.current.silentStart).toBe(true);
    expect(setLastMessage).toHaveBeenCalledWith("当前是浏览器预览模式，启动设置仅用于界面预览。");
  });

  it("桌面端：handleSilentStartChange 调用 save 并更新", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ launch_at_startup: false, silent_start: false })
      .mockResolvedValueOnce({ launch_at_startup: false, silent_start: true });
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.startupSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleSilentStartChange(true);
    });

    expect(callCommand).toHaveBeenCalledWith("save_startup_settings", {
      launchAtStartup: false,
      silentStart: true,
    });
    expect(result.current.silentStart).toBe(true);
    expect(setLastMessage).toHaveBeenCalledWith("已开启静默启动，下次启动时将只驻留托盘。");
  });

  it("桌面端：handleSilentStartChange 保存失败时显示错误", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ launch_at_startup: false, silent_start: false })
      .mockRejectedValueOnce(new Error("save failed"));
    const { result } = renderHook(() =>
      useStartupSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.startupSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleSilentStartChange(true);
    });

    expect(setLastMessage).toHaveBeenCalledWith("这次没有保存成功，请稍后再试。");
    expect(result.current.isSavingStartupSettings).toBe(false);
  });
});
