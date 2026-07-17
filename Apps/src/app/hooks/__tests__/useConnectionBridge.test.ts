import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { emitTauriEvent } from "@/test/tauri-mock";

import { useConnectionBridge } from "../useConnectionBridge";
import type { ConnectedPeer, LanDevicePayload, ShellBootstrapPayload, ShellDeferredPayload, TrustedPeerPayload } from "../../types";

vi.mock("../../utils/scheduleDeferred", () => ({
  scheduleDeferred: (work: () => void) => { work(); },
}));

const BOOTSTRAP: ShellBootstrapPayload = {
  pairing_code: "123456",
  status: "connected",
  connected_peers: [],
  ui_settings: { color_scheme: "dark", theme_color: "cyan", device_name: "MyPC" },
  pending_connection_request: null,
  presence_enabled: true,
};

const DEFERRED: ShellDeferredPayload = {
  lan_devices: [],
  trusted_peers: [],
  clipboard_history: [],
};

function makeBridge(overrides: {
  tauriAvailable?: boolean;
  status?: string;
  callCommand?: ReturnType<typeof vi.fn>;
  callCommandImpl?: (cmd: string) => unknown;
} = {}) {
  const callCommand = overrides.callCommand ?? vi.fn();
  if (overrides.callCommandImpl) {
    callCommand.mockImplementation(overrides.callCommandImpl);
  }
  const mocks = {
    callCommand,
    pairingStageRef: { current: "idle" as const },
    setStatus: vi.fn(),
    setLastMessage: vi.fn(),
    showNotice: vi.fn(),
    onSyncActivity: vi.fn(),
    setPairingCode: vi.fn(),
    onPairingCodeRotated: vi.fn(),
    setLanDevices: vi.fn(),
    setTrustedPeers: vi.fn(),
    setClips: vi.fn(),
    setConnectedPeers: vi.fn(),
    applyDesktopUiSettings: vi.fn(),
    applyUiSettingsFallback: vi.fn(),
    toUserMessage: vi.fn((_: unknown, fallback: string) => fallback),
    onConnectionRequest: vi.fn(),
    onConnectionEstablished: vi.fn(),
    onConnectionFailed: vi.fn(),
    onConnectionEnded: vi.fn(),
    onOutboundConnectionStarted: vi.fn(),
    onOutboundConnectionPending: vi.fn(),
    onOutboundConnectionSettled: vi.fn(),
    onPairingCodeNeeded: vi.fn(),
    onBackendConnectionSynced: vi.fn(),
    onShellDeferred: vi.fn(),
  };
  const hook = renderHook(() => useConnectionBridge({
    tauriAvailable: overrides.tauriAvailable ?? false,
    callCommand: mocks.callCommand,
    status: (overrides.status ?? "offline") as any,
    connectedPeers: [],
    pairingStageRef: mocks.pairingStageRef,
    setStatus: mocks.setStatus,
    setLastMessage: mocks.setLastMessage,
    showNotice: mocks.showNotice,
    onSyncActivity: mocks.onSyncActivity,
    setPairingCode: mocks.setPairingCode,
    onPairingCodeRotated: mocks.onPairingCodeRotated,
    setLanDevices: mocks.setLanDevices,
    setTrustedPeers: mocks.setTrustedPeers,
    setClips: mocks.setClips,
    setConnectedPeers: mocks.setConnectedPeers,
    applyDesktopUiSettings: mocks.applyDesktopUiSettings,
    applyUiSettingsFallback: mocks.applyUiSettingsFallback,
    toUserMessage: mocks.toUserMessage,
    onConnectionRequest: mocks.onConnectionRequest,
    onConnectionEstablished: mocks.onConnectionEstablished,
    onConnectionFailed: mocks.onConnectionFailed,
    onConnectionEnded: mocks.onConnectionEnded,
    onOutboundConnectionStarted: mocks.onOutboundConnectionStarted,
    onOutboundConnectionPending: mocks.onOutboundConnectionPending,
    onOutboundConnectionSettled: mocks.onOutboundConnectionSettled,
    onPairingCodeNeeded: mocks.onPairingCodeNeeded,
    onBackendConnectionSynced: mocks.onBackendConnectionSynced,
    onShellDeferred: mocks.onShellDeferred,
  }));
  return { mocks, ...hook };
}

describe("useConnectionBridge", () => {
  it("tauriAvailable=false 时设置预览模式状态", () => {
    const { mocks } = makeBridge({ tauriAvailable: false });
    expect(mocks.setPairingCode).toHaveBeenCalledWith("桌面端可用");
    expect(mocks.setStatus).toHaveBeenCalledWith("offline");
    expect(mocks.setLastMessage).toHaveBeenCalledWith("当前是浏览器预览模式，连接能力需在桌面应用中体验。");
  });

  it("tauriAvailable=true 时 bootstrap 成功，设置状态", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return BOOTSTRAP;
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setPairingCode).toHaveBeenCalledWith("123456"); });
    expect(mocks.applyDesktopUiSettings).toHaveBeenCalledWith(BOOTSTRAP.ui_settings);
    expect(mocks.setStatus).toHaveBeenCalledWith("online");
    expect(mocks.setLastMessage).toHaveBeenCalledWith("已恢复现有连接，可以继续同步剪贴板。");
  });

  it("tauriAvailable=true 时 bootstrap 失败，回退到 offline", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: () => { throw new Error("bootstrap failed"); },
    });

    await waitFor(() => { expect(mocks.setStatus).toHaveBeenCalledWith("offline"); });
    expect(mocks.applyUiSettingsFallback).toHaveBeenCalled();
    expect(mocks.setLastMessage).toHaveBeenCalledWith("连接桥接初始化失败，请稍后再试。");
  });

  it("bootstrap 含 pending_connection_request 时触发 onConnectionRequest", async () => {
    const connReq = { peer_name: "TestPC", peer_id: "abc", address: "192.168.1.2", port: 19876 };
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return { ...BOOTSTRAP, pending_connection_request: connReq };
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.onConnectionRequest).toHaveBeenCalledWith(connReq); });
  });

  it("bootstrap 含 connected_peers 时更新 peers", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return { ...BOOTSTRAP, connected_peers: [{ peer_name: "Peer1", peer_id: "p1" }] };
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setConnectedPeers).toHaveBeenCalled(); });
  });

  it("bootstrap 含 disconnected 状态时设置 offline", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return { ...BOOTSTRAP, status: "disconnected" };
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setStatus).toHaveBeenCalledWith("offline"); });
    expect(mocks.setLastMessage).toHaveBeenCalledWith("正在监听设备列表与连接请求。");
  });

  it("scheduleDeferred 加载 deferred 数据成功", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return BOOTSTRAP;
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setLanDevices).toHaveBeenCalledWith(DEFERRED.lan_devices); });
    expect(mocks.setTrustedPeers).toHaveBeenCalledWith(DEFERRED.trusted_peers);
    expect(mocks.onShellDeferred).toHaveBeenCalledWith(DEFERRED);
  });

  it("scheduleDeferred 加载失败时显示错误", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return BOOTSTRAP;
        if (cmd === "get_shell_deferred") throw new Error("deferred failed");
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setLastMessage).toHaveBeenCalledWith("部分数据加载较慢，请稍后刷新。"); });
  });

  it("clipboard-sync-activity 事件触发 setLastMessage 和 onSyncActivity", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return BOOTSTRAP;
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setPairingCode).toHaveBeenCalledWith("123456"); });
    emitTauriEvent("clipboard-sync-activity", { active: true, kind: "file", message: "传输中…" });
    expect(mocks.setLastMessage).toHaveBeenCalledWith("传输中…");
    expect(mocks.onSyncActivity).toHaveBeenCalledWith({ active: true, kind: "file", message: "传输中…" });
  });

  it("lan-devices-changed 事件更新设备列表", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return BOOTSTRAP;
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setPairingCode).toHaveBeenCalledWith("123456"); });
    const devices: LanDevicePayload[] = [{ device_name: "Peer1", host: "192.168.1.2", port: 19876, os: "windows" }];
    emitTauriEvent("lan-devices-changed", devices);
    expect(mocks.setLanDevices).toHaveBeenCalled();
  });

  it("pairing-code-rotated 事件更新配对码", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return BOOTSTRAP;
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setPairingCode).toHaveBeenCalledWith("123456"); });
    emitTauriEvent("pairing-code-rotated", { code: "654321", expires_at: 0 });
    expect(mocks.setPairingCode).toHaveBeenCalledWith("654321");
    expect(mocks.onPairingCodeRotated).toHaveBeenCalledWith({ code: "654321", expires_at: 0 });
  });

  it("connection-request 事件触发回调", async () => {
    const { mocks } = makeBridge({
      tauriAvailable: true,
      callCommandImpl: (cmd) => {
        if (cmd === "get_shell_bootstrap") return BOOTSTRAP;
        if (cmd === "get_shell_deferred") return DEFERRED;
        return undefined;
      },
    });

    await waitFor(() => { expect(mocks.setPairingCode).toHaveBeenCalledWith("123456"); });
    emitTauriEvent("connection-request", { peer_name: "Peer", peer_id: "p1", address: "192.168.1.2", port: 19876 });
    expect(mocks.onConnectionRequest).toHaveBeenCalled();
  });
});
