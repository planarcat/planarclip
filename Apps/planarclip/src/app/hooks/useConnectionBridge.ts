import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef } from "react";
import type {
  AppConnectionStatus,
  ClipboardHistoryPayload,
  CommandExecutor,
  ConnectedPeer,
  ConnectionEndedPayload,
  ConnectionEstablishedPayload,
  ConnectionFailedPayload,
  ConnectionRequestPayload,
  LanDevicePayload,
  PairingCodeRotatedPayload,
  PairingStage,
  TrustedPeerPayload,
  UiSettingsPayload,
} from "../types";
import { mapClipboardHistory } from "../utils/clipboard";

type UseConnectionBridgeOptions = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  status: AppConnectionStatus;
  connectedPeer: ConnectedPeer | null;
  pairingStageRef: React.MutableRefObject<PairingStage>;
  setStatus: (status: AppConnectionStatus) => void;
  setLastMessage: (message: string) => void;
  setPairingCode: (pairingCode: string) => void;
  onPairingCodeRotated?: (payload: PairingCodeRotatedPayload) => void;
  setLanDevices: (devices: LanDevicePayload[]) => void;
  setTrustedPeers: (peers: TrustedPeerPayload[]) => void;
  setClips: (clips: ReturnType<typeof mapClipboardHistory>) => void;
  setConnectedPeer: (peer: ConnectedPeer | null) => void;
  applyDesktopUiSettings: (settings: UiSettingsPayload) => void;
  applyUiSettingsFallback: () => void;
  toUserMessage: (error: unknown, fallback: string, targetName?: string) => string;
  onConnectionRequest: (payload: ConnectionRequestPayload) => void;
  onConnectionEstablished: (payload: ConnectionEstablishedPayload) => void;
  onConnectionFailed: (payload: ConnectionFailedPayload) => void;
  onConnectionEnded: (payload: ConnectionEndedPayload) => void;
};

/**
 * 管理桌面桥接初始化、事件订阅与连接状态轮询。
 * 输入：连接相关状态 setter、配对事件回调与外观设置同步方法。
 * 输出：无；该 hook 通过副作用驱动桌面端状态进入前端界面。
 */
export function useConnectionBridge({
  tauriAvailable,
  callCommand,
  status,
  connectedPeer,
  pairingStageRef,
  setStatus,
  setLastMessage,
  setPairingCode,
  onPairingCodeRotated,
  setLanDevices,
  setTrustedPeers,
  setClips,
  setConnectedPeer,
  applyDesktopUiSettings,
  applyUiSettingsFallback,
  toUserMessage,
  onConnectionRequest,
  onConnectionEstablished,
  onConnectionFailed,
  onConnectionEnded,
}: UseConnectionBridgeOptions) {
  const statusRef = useRef(status);
  const connectedPeerRef = useRef(connectedPeer);

  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  useEffect(() => {
    connectedPeerRef.current = connectedPeer;
  }, [connectedPeer]);

  const refreshConnectionStatus = useCallback(async () => {
    try {
      const connectionStatus = await callCommand<string>("get_status");
      const isConnected = connectionStatus === "connected";

      if (isConnected) {
        if (statusRef.current !== "online") {
          setStatus("online");
        }
        return;
      }

      if (pairingStageRef.current === "idle") {
        if (connectedPeerRef.current) {
          setConnectedPeer(null);
          setLastMessage("当前连接已断开，请重新连接。");
        }
        setStatus("offline");
      }
    } catch {
      if (pairingStageRef.current === "idle") {
        setStatus("offline");
      }
    }
  }, [callCommand, pairingStageRef, setConnectedPeer, setLastMessage, setStatus]);

  useEffect(() => {
    if (!tauriAvailable) {
      setPairingCode("桌面端可用");
      setStatus("offline");
      setLastMessage("当前是浏览器预览模式，连接能力需在桌面应用中体验。");
      return;
    }

    let disposed = false;
    let eventCleanup: Array<() => void> = [];

    const setup = async () => {
      try {
        const [
          initialStatus,
          initialPairingCode,
          initialLanDevices,
          initialTrustedPeers,
          initialUiSettings,
          initialClipboardHistory,
        ] = await Promise.all([
          callCommand<string>("get_status"),
          callCommand<string>("get_pairing_code"),
          callCommand<LanDevicePayload[]>("get_lan_devices"),
          callCommand<TrustedPeerPayload[]>("get_trusted_peers"),
          callCommand<UiSettingsPayload>("get_ui_settings"),
          callCommand<ClipboardHistoryPayload[]>("get_clipboard_history"),
        ]);

        if (disposed) {
          return;
        }

        setPairingCode(initialPairingCode);
        setLanDevices(initialLanDevices);
        setTrustedPeers(initialTrustedPeers);
        setClips(mapClipboardHistory(initialClipboardHistory));
        applyDesktopUiSettings(initialUiSettings);
        setStatus(initialStatus === "connected" ? "online" : "offline");
        setLastMessage(
          initialStatus === "connected"
            ? "已恢复现有连接，可以继续同步剪贴板。"
            : "正在监听设备列表与连接请求。",
        );
      } catch (error) {
        if (!disposed) {
          setStatus("offline");
          applyUiSettingsFallback();
          setLastMessage(toUserMessage(error, "连接桥接初始化失败，请稍后再试。"));
        }
      }

      const listeners = await Promise.all([
        listen<LanDevicePayload[]>("lan-devices-changed", (event) => {
          setLanDevices(event.payload);
        }),
        listen<ClipboardHistoryPayload[]>("clipboard-history-changed", (event) => {
          setClips(mapClipboardHistory(event.payload));
        }),
        listen<ConnectionRequestPayload>("connection-request", (event) => {
          onConnectionRequest(event.payload);
        }),
        listen<ConnectionEstablishedPayload>("connection-established", (event) => {
          onConnectionEstablished(event.payload);
        }),
        listen<ConnectionFailedPayload>("connection-failed", (event) => {
          onConnectionFailed(event.payload);
        }),
        listen<ConnectionEndedPayload>("connection-ended", (event) => {
          onConnectionEnded(event.payload);
        }),
        listen<PairingCodeRotatedPayload>("pairing-code-rotated", (event) => {
          setPairingCode(event.payload.code);
          onPairingCodeRotated?.(event.payload);
        }),
      ]);

      if (disposed) {
        listeners.forEach((unlisten) => unlisten());
        return;
      }

      eventCleanup = listeners;
    };

    void setup();

    const timer = window.setInterval(() => {
      void refreshConnectionStatus();
    }, 5_000);

    return () => {
      disposed = true;
      window.clearInterval(timer);
      eventCleanup.forEach((unlisten) => unlisten());
    };
  }, [
    applyDesktopUiSettings,
    applyUiSettingsFallback,
    callCommand,
    onConnectionEnded,
    onConnectionEstablished,
    onConnectionFailed,
    onConnectionRequest,
    onPairingCodeRotated,
    refreshConnectionStatus,
    setClips,
    setLanDevices,
    setLastMessage,
    setPairingCode,
    setStatus,
    setTrustedPeers,
    tauriAvailable,
    toUserMessage,
  ]);
}
