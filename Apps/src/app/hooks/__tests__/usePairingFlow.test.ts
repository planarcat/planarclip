import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { usePairingFlow } from "../usePairingFlow";
import type { ConnectedPeer, ConnectionRequestPayload, Device } from "../../types";

vi.mock("../../utils/device", () => ({
  formatDeviceAddress: (host: string, port: number) => `${host}:${port}`,
  inferOs: (name: string) => name.includes("Mac") ? "macos" : "windows",
}));
vi.mock("../../utils/message", () => ({
  MSG_CONNECTION_LIMIT: "连接数已达上限，请先断开其他连接。",
  MSG_INVALID_PAIRING_CODE: "配对码无效，请重新输入。",
  MSG_PAIRING_CODE_REFRESHED: "配对码已更新，请使用新码。",
  MSG_ENTER_PEER_PAIRING_CODE: "请输入对方设备上的配对码。",
  MSG_WAIT_FOR_PEER_PAIRING_CODE: "等待对方确认配对码…",
  MSG_PEER_CANCELLED: "对方已取消连接。",
  MSG_PEER_REJECTED: "对方拒绝了连接请求。",
  MSG_PEER_RESPONSE_TIMEOUT: "对方未及时响应，已取消连接。",
  MSG_SELF_CANCELLED_INBOUND: "已取消入站连接。",
  MSG_SELF_CANCELLED_OUTBOUND: "已取消连接请求。",
  MSG_SELF_INCOMING_TIMEOUT: "未及时回应，连接请求已超时。",
  connectionUnavailableMessage: (name?: string) => name ? `无法连接 ${name}` : "无法连接该设备",
  isConnectionRejected: (error: any) => error?.kind === "rejected" || error?.kind === "cancelled",
  isConnectionTimeout: (error: any) => error?.kind === "timeout",
  isInvalidPairingCode: (error: any) => error?.kind === "invalid_pairing_code",
  isPeerCancelled: (payload: any) => payload?.kind === "peer_cancelled",
  isPeerOffline: (payload: any) => payload?.kind === "peer_offline",
  normalizeUserMessage: (error: any, fallback: string, _name?: string) => {
    if (error?.kind === "rejected") return "对方拒绝了连接请求。";
    if (error?.kind === "cancelled") return "对方已取消连接。";
    if (error?.kind === "timeout") return "对方未及时响应，已取消连接。";
    if (error?.kind === "invalid_pairing_code") return "配对码无效，请重新输入。";
    if (error?.kind === "peer_offline") return "对方已下线。";
    return fallback;
  },
  connectionEndedMessage: (payload: any) => payload?.message || "连接已断开。",
}));
vi.mock("../../utils/time", () => ({
  formatTime: () => "12:00:00",
}));

const DEVICE: Device = {
  id: "lan:peer1",
  name: "TestPC",
  os: "windows",
  host: "192.168.1.2",
  port: 19876,
  peerId: "p1",
  address: "192.168.1.2:19876",
  status: "idle",
  source: "lan",
  isTrusted: false,
  discoveredOnLan: true,
};

const CONNECTED: ConnectedPeer = {
  name: "ConnectedPC",
  peerId: "p1",
  address: "192.168.1.3:19876",
  os: "windows",
  source: "lan",
};

function makePairing(overrides: {
  callCommand?: ReturnType<typeof vi.fn>;
  callCommandImpl?: (cmd: string, args?: unknown) => unknown;
  status?: string;
  connectedCount?: number;
  pairingInput?: string;
  pairingStage?: string;
  pairingTarget?: Device | null;
  incomingRequest?: ConnectionRequestPayload | null;
} = {}) {
  const callCommand = overrides.callCommand ?? vi.fn();
  if (overrides.callCommandImpl) {
    callCommand.mockImplementation(overrides.callCommandImpl);
  }
  const mocks = {
    callCommand,
    setStatus: vi.fn(),
    setLastMessage: vi.fn(),
    setConnectedPeers: vi.fn(),
    setShowPairing: vi.fn(),
    setPairingInput: vi.fn(),
    setPairingStage: vi.fn(),
    setPairingTarget: vi.fn(),
    setPairingHelperText: vi.fn(),
    setPairingError: vi.fn(),
    setPairingRotationHint: vi.fn(),
    setPairingCode: vi.fn(),
    setIncomingRequest: vi.fn(),
    refreshLanDevices: vi.fn(),
    showNotice: vi.fn(),
  };
  const hook = renderHook(() => usePairingFlow({
    callCommand: mocks.callCommand,
    status: (overrides.status ?? "offline") as any,
    connectedCount: overrides.connectedCount ?? 0,
    pairingInput: overrides.pairingInput ?? "",
    pairingStage: (overrides.pairingStage ?? "idle") as any,
    pairingTarget: overrides.pairingTarget ?? null,
    incomingRequest: overrides.incomingRequest ?? null,
    setStatus: mocks.setStatus,
    setLastMessage: mocks.setLastMessage,
    setConnectedPeers: mocks.setConnectedPeers,
    setShowPairing: mocks.setShowPairing,
    setPairingInput: mocks.setPairingInput,
    setPairingStage: mocks.setPairingStage,
    setPairingTarget: mocks.setPairingTarget,
    setPairingHelperText: mocks.setPairingHelperText,
    setPairingError: mocks.setPairingError,
    setPairingRotationHint: mocks.setPairingRotationHint,
    setPairingCode: mocks.setPairingCode,
    setIncomingRequest: mocks.setIncomingRequest,
    refreshLanDevices: mocks.refreshLanDevices,
    showNotice: mocks.showNotice,
  }));
  return { mocks, ...hook };
}

describe("usePairingFlow", () => {
  describe("openPairingModal", () => {
    it("打开配对弹层，设置 idle 状态", async () => {
      const { mocks, result } = makePairing({ callCommandImpl: (cmd) => {
        if (cmd === "rotate_pairing_code") return { code: "123456", expires_at: 0 };
        return undefined;
      }});

      await act(async () => { result.current.openPairingModal(); });
      expect(mocks.setPairingTarget).toHaveBeenCalledWith(null);
      expect(mocks.setPairingStage).toHaveBeenCalledWith("idle");
      expect(mocks.setShowPairing).toHaveBeenCalledWith(true);
    });

    it("达到连接上限时提示", async () => {
      const { mocks, result } = makePairing({ connectedCount: 10 });
      await act(async () => { result.current.openPairingModal(); });
      expect(mocks.showNotice).toHaveBeenCalledWith("连接数已达上限，请先断开其他连接。");
    });
  });

  describe("closePairingModal", () => {
    it("idle 阶段关闭弹层并重置", async () => {
      const { mocks, result } = makePairing({ pairingStage: "idle" });
      await act(async () => { result.current.closePairingModal(); });
      expect(mocks.setShowPairing).toHaveBeenCalledWith(false);
    });

    it("incoming_pairing 阶段取消入站连接", async () => {
      const { mocks, result } = makePairing({ pairingStage: "incoming_pairing" });
      await act(async () => { result.current.closePairingModal(); });
      expect(mocks.callCommand).toHaveBeenCalledWith("reject_connection");
      expect(mocks.setLastMessage).toHaveBeenCalledWith("已取消入站连接。");
    });

    it("requesting_device 阶段取消出站连接", async () => {
      const { mocks, result } = makePairing({
        pairingStage: "requesting_device",
        callCommandImpl: (cmd) => {
          if (cmd === "abort_connection") return undefined;
          return undefined;
        },
      });
      await act(async () => { result.current.closePairingModal(); });
      expect(mocks.setLastMessage).toHaveBeenCalledWith("已取消连接请求。");
    });
  });

  describe("handleConnectLan", () => {
    it("已连接设备跳过", async () => {
      const { mocks, result } = makePairing();
      await act(async () => { result.current.handleConnectLan({ ...DEVICE, status: "connected" }); });
      expect(mocks.callCommand).not.toHaveBeenCalled();
    });

    it("连接成功: 直接建立连接", async () => {
      const { mocks, result } = makePairing({
        callCommandImpl: (cmd) => {
          if (cmd === "connect_lan") return "connected";
          if (cmd === "rotate_pairing_code") return { code: "123456", expires_at: 0 };
          return undefined;
        },
      });
      await act(async () => { result.current.handleConnectLan(DEVICE); });
      expect(mocks.callCommand).toHaveBeenCalledWith("connect_lan", {
        ip: "192.168.1.2", port: 19876, peerId: "p1",
      });
      expect(mocks.setStatus).toHaveBeenCalledWith("online");
    });

    it("连接返回 awaiting_code: 显示配对码输入", async () => {
      const { mocks, result } = makePairing({
        callCommandImpl: (cmd) => {
          if (cmd === "connect_lan") return "awaiting_code";
          if (cmd === "rotate_pairing_code") return { code: "123456", expires_at: 0 };
          return undefined;
        },
      });
      await act(async () => { result.current.handleConnectLan(DEVICE); });
      expect(mocks.setPairingStage).toHaveBeenCalledWith("awaiting_code");
      expect(mocks.setShowPairing).toHaveBeenCalledWith(true);
    });

    it("连接失败: 显示错误", async () => {
      const { mocks, result } = makePairing({
        callCommandImpl: (cmd) => {
          if (cmd === "connect_lan") throw new Error("connection refused");
          return undefined;
        },
      });
      await act(async () => { result.current.handleConnectLan(DEVICE); });
      expect(mocks.setPairingStage).toHaveBeenCalledWith("error");
    });

    it("达到连接上限时提示", async () => {
      const { mocks, result } = makePairing({ connectedCount: 10 });
      await act(async () => { result.current.handleConnectLan(DEVICE); });
      expect(mocks.showNotice).toHaveBeenCalledWith("连接数已达上限，请先断开其他连接。");
    });

    it("缺少 host/port 时显示错误", async () => {
      const { mocks, result } = makePairing();
      await act(async () => { result.current.handleConnectLan({ ...DEVICE, host: undefined, port: undefined }); });
      expect(mocks.setPairingError).toHaveBeenCalledWith("当前设备缺少连接地址，请等待下一轮发现结果。");
    });
  });

  describe("handleSubmitPairingCode", () => {
    it("输入不足 6 位时显示错误", async () => {
      const { mocks, result } = makePairing({ pairingInput: "123" });
      await act(async () => { result.current.handleSubmitPairingCode(); });
      expect(mocks.setPairingError).toHaveBeenCalledWith("请输入 6 位数字配对码。");
    });

    it("提交配对码成功（出站）", async () => {
      const { mocks, result } = makePairing({
        pairingInput: "123456",
        pairingStage: "awaiting_code",
        callCommandImpl: (cmd) => {
          if (cmd === "submit_pairing_code") return "verified";
          return undefined;
        },
      });
      await act(async () => { result.current.handleSubmitPairingCode(); });
      expect(mocks.callCommand).toHaveBeenCalledWith("submit_pairing_code", { code: "123456" });
    });

    it("配对码无效", async () => {
      const { mocks, result } = makePairing({
        pairingInput: "000000",
        pairingStage: "awaiting_code",
        callCommandImpl: (cmd) => {
          if (cmd === "submit_pairing_code") throw { kind: "invalid_pairing_code" };
          return undefined;
        },
      });
      await act(async () => { result.current.handleSubmitPairingCode(); });
      expect(mocks.setPairingError).toHaveBeenCalledWith("配对码无效，请重新输入。");
    });
  });

  describe("handleRotatePairingCode", () => {
    it("在 awaiting_code 阶段刷新配对码", async () => {
      const { mocks, result } = makePairing({
        pairingStage: "awaiting_code",
        callCommandImpl: (cmd) => {
          if (cmd === "rotate_pairing_code") return { code: "654321", expires_at: 0 };
          return undefined;
        },
      });
      await act(async () => { result.current.handleRotatePairingCode(); });
      expect(mocks.callCommand).toHaveBeenCalledWith("rotate_pairing_code");
    });

    it("在 idle 阶段跳过", async () => {
      const { mocks, result } = makePairing({ pairingStage: "idle" });
      await act(async () => { result.current.handleRotatePairingCode(); });
      expect(mocks.callCommand).not.toHaveBeenCalled();
    });
  });

  describe("handleAcceptIncoming", () => {
    it("接受入站连接，无需配对", async () => {
      const { mocks, result } = makePairing({
        incomingRequest: { device_name: "Peer", peer_id: "p3", address: "192.168.1.4", port: 19876, requires_pairing: false },
        callCommandImpl: (cmd) => {
          if (cmd === "accept_connection") return undefined;
          return undefined;
        },
      });
      await act(async () => { result.current.handleAcceptIncoming(); });
      expect(mocks.callCommand).toHaveBeenCalledWith("accept_connection");
    });

    it("无 incomingRequest 时跳过", async () => {
      const { mocks, result } = makePairing({ incomingRequest: null });
      await act(async () => { result.current.handleAcceptIncoming(); });
      expect(mocks.callCommand).not.toHaveBeenCalled();
    });
  });

  describe("handleRejectIncoming", () => {
    it("拒绝入站连接", async () => {
      const { mocks, result } = makePairing();
      await act(async () => { result.current.handleRejectIncoming(); });
      expect(mocks.callCommand).toHaveBeenCalledWith("reject_connection");
    });
  });

  describe("handleIncomingResponseTimeout", () => {
    it("入站响应超时", async () => {
      const { mocks, result } = makePairing();
      await act(async () => { result.current.handleIncomingResponseTimeout(); });
      expect(mocks.callCommand).toHaveBeenCalledWith("timeout_incoming_connection");
    });
  });

  describe("handleDisconnect", () => {
    it("断开指定设备", async () => {
      const { mocks, result } = makePairing({
        callCommandImpl: (cmd) => {
          if (cmd === "disconnect_peer") return undefined;
          return undefined;
        },
      });
      await act(async () => { result.current.handleDisconnect(CONNECTED); });
      expect(mocks.callCommand).toHaveBeenCalledWith("disconnect_peer", { peerId: "p1" });
    });

    it("断开所有连接", async () => {
      const { mocks, result } = makePairing({
        callCommandImpl: (cmd) => {
          if (cmd === "disconnect") return undefined;
          return undefined;
        },
      });
      await act(async () => { result.current.handleDisconnect(); });
      expect(mocks.callCommand).toHaveBeenCalledWith("disconnect");
    });
  });

  describe("handleConnectionRequest", () => {
    it("陌生设备请求配对", () => {
      const { mocks, result } = makePairing();
      act(() => {
        result.current.handleConnectionRequest({
          device_name: "NewPC",
          peer_id: "p4",
          address: "192.168.1.5",
          port: 19876,
          requires_pairing: true,
        });
      });
      expect(mocks.setPairingStage).toHaveBeenCalledWith("incoming_request");
    });
  });

  describe("handleConnectionEstablished", () => {
    it("连接建立后更新 peers 和状态", () => {
      const { mocks, result } = makePairing();
      act(() => {
        result.current.handleConnectionEstablished({
          peer_name: "NewPeer", peer_id: "p6", is_reconnect: false,
        });
      });
      expect(mocks.setStatus).toHaveBeenCalledWith("online");
    });

    it("重连时显示简短消息", () => {
      const { mocks, result } = makePairing();
      act(() => {
        result.current.handleConnectionEstablished({
          peer_name: "ReconnectPeer", peer_id: "p7", is_reconnect: true,
        });
      });
      expect(mocks.setLastMessage).toHaveBeenCalledWith("ReconnectPeer 已连接");
    });
  });

  describe("handleConnectionFailed", () => {
    it("配对阶段对方取消", () => {
      const { mocks, result } = makePairing({ pairingStage: "awaiting_code" });
      act(() => {
        result.current.handleConnectionFailed({ kind: "peer_cancelled", message: "" });
      });
      expect(mocks.setLastMessage).toHaveBeenCalledWith("对方已取消连接。");
    });

    it("配对码无效时提示重新输入", () => {
      const { mocks, result } = makePairing({ pairingStage: "awaiting_code" });
      act(() => {
        result.current.handleConnectionFailed({ kind: "invalid_pairing_code", message: "" });
      });
      expect(mocks.setPairingError).toHaveBeenCalledWith("配对码无效，请重新输入。");
    });
  });

  describe("handleConnectionEnded", () => {
    it("更新 peers 并显示断开消息", async () => {
      const { mocks, result } = makePairing();
      await act(async () => {
        result.current.handleConnectionEnded({ peer_id: "p2", message: "连接已断开。" });
      });
      expect(mocks.setConnectedPeers).toHaveBeenCalled();
    });
  });

  describe("handleOutboundConnectionStarted", () => {
    it("开始出站连接", () => {
      const { mocks, result } = makePairing();
      act(() => {
        result.current.handleOutboundConnectionStarted({
          peer_id: "p8", peer_name: "OutboundPC", peer_ip: "192.168.1.7", peer_port: 19876,
        });
      });
      expect(mocks.setPairingStage).toHaveBeenCalledWith("requesting_device");
    });
  });

  describe("handleOutboundConnectionPending", () => {
    it("出站连接等待中", () => {
      const { mocks, result } = makePairing();
      act(() => {
        result.current.handleOutboundConnectionPending({
          peer_id: "p9", peer_name: "PendingPC", peer_ip: "192.168.1.8", peer_port: 19876,
        });
      });
      expect(mocks.setPairingStage).toHaveBeenCalledWith("requesting_device");
    });
  });

  describe("handleOutboundConnectionSettled", () => {
    it("出站连接已结算，清除 UI", () => {
      const { mocks, result } = makePairing({ pairingStage: "requesting_device" });
      act(() => {
        result.current.handleOutboundConnectionSettled({ peer_id: "p1", success: true });
      });
      expect(mocks.setPairingStage).toHaveBeenCalledWith("idle");
    });
  });

  describe("syncOutboundAttemptWithBackend", () => {
    it("连接已建立时清除出站尝试", () => {
      const { mocks, result } = makePairing({
        pairingStage: "requesting_device",
        pairingTarget: DEVICE,
      });
      act(() => {
        result.current.syncOutboundAttemptWithBackend([CONNECTED]);
      });
      expect(mocks.setPairingStage).toHaveBeenCalledWith("idle");
    });
  });

  describe("connectionLocked", () => {
    it("在出站阶段为 true", () => {
      const { result } = makePairing({ pairingStage: "awaiting_code" });
      expect(result.current.connectionLocked).toBe(true);
    });

    it("在 idle 阶段为 false", () => {
      const { result } = makePairing({ pairingStage: "idle" });
      expect(result.current.connectionLocked).toBe(false);
    });
  });
});
