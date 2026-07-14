import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useConnectionSettings } from "../useConnectionSettings";

function makeMocks() {
  const setLastMessage = vi.fn();
  const callCommand = vi.fn();
  return { setLastMessage, callCommand };
}

describe("useConnectionSettings", () => {
  it("浏览器预览模式：使用默认值，loaded 直接为 true", () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useConnectionSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    expect(result.current.autoConnectTrusted).toBe(false);
    expect(result.current.connectionSettingsLoaded).toBe(true);
    expect(result.current.isSavingConnectionSettings).toBe(false);
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("桌面端加载成功：从 callCommand 获取设置", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockResolvedValue({ auto_connect_trusted: true });
    const { result } = renderHook(() =>
      useConnectionSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.connectionSettingsLoaded).toBe(true);
    });
    expect(result.current.autoConnectTrusted).toBe(true);
  });

  it("桌面端加载失败：回退到默认值", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand.mockRejectedValue(new Error("network error"));
    const { result } = renderHook(() =>
      useConnectionSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.connectionSettingsLoaded).toBe(true);
    });
    expect(result.current.autoConnectTrusted).toBe(false);
  });

  it("浏览器预览模式：handleAutoConnectTrustedChange 仅更新本地状态", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    const { result } = renderHook(() =>
      useConnectionSettings({
        tauriAvailable: false,
        callCommand,
        setLastMessage,
      }),
    );

    await act(async () => {
      result.current.handleAutoConnectTrustedChange(true);
    });

    expect(result.current.autoConnectTrusted).toBe(true);
    expect(setLastMessage).toHaveBeenCalledWith(
      "当前是浏览器预览模式，连接设置仅用于界面预览。",
    );
    expect(callCommand).not.toHaveBeenCalled();
  });

  it("桌面端：handleAutoConnectTrustedChange 调用 save 并更新", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ auto_connect_trusted: false })
      .mockResolvedValueOnce({ auto_connect_trusted: true });
    const { result } = renderHook(() =>
      useConnectionSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.connectionSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleAutoConnectTrustedChange(true);
    });

    expect(callCommand).toHaveBeenCalledWith("save_connection_settings", {
      autoConnectTrusted: true,
    });
    expect(result.current.autoConnectTrusted).toBe(true);
    expect(setLastMessage).toHaveBeenCalledWith("已开启自动连接熟悉设备，启动或发现设备上线时会自动发起连接。");
  });

  it("桌面端：handleAutoConnectTrustedChange 保存失败时显示错误", async () => {
    const { callCommand, setLastMessage } = makeMocks();
    callCommand
      .mockResolvedValueOnce({ auto_connect_trusted: false })
      .mockRejectedValueOnce(new Error("save failed"));
    const { result } = renderHook(() =>
      useConnectionSettings({
        tauriAvailable: true,
        callCommand,
        setLastMessage,
      }),
    );

    await waitFor(() => {
      expect(result.current.connectionSettingsLoaded).toBe(true);
    });

    await act(async () => {
      result.current.handleAutoConnectTrustedChange(true);
    });

    expect(setLastMessage).toHaveBeenCalledWith("这次没有保存成功，请稍后再试。");
    expect(result.current.isSavingConnectionSettings).toBe(false);
  });
});
