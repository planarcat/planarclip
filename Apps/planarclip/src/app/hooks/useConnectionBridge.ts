import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, type Dispatch, type SetStateAction } from "react";
import type {
  AppConnectionStatus,
  ClipboardHistoryPayload,
  CommandExecutor,
  ConnectedPeer,
  ConnectedPeerPayload,
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
import { areLanDevicesEqual, inferOs } from "../utils/device";

function mapConnectedPeerPayload(payload: ConnectedPeerPayload): ConnectedPeer {
  const peerName = payload.peer_name || "已连接设备";
  return {
    name: peerName,
    peerId: payload.peer_id,
    address: "局域网直连",
    os: inferOs(peerName),
    source: "lan",
  };
}

function useLatestRef<T>(value: T) {
  const ref = useRef(value);
  ref.current = value;
  return ref;
}

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
  setLanDevices: Dispatch<SetStateAction<LanDevicePayload[]>>;
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

  const setLanDevicesRef = useLatestRef(setLanDevices);
  const setClipsRef = useLatestRef(setClips);
  const setPairingCodeRef = useLatestRef(setPairingCode);
  const setStatusRef = useLatestRef(setStatus);
  const setLastMessageRef = useLatestRef(setLastMessage);
  const setTrustedPeersRef = useLatestRef(setTrustedPeers);
  const setConnectedPeerRef = useLatestRef(setConnectedPeer);
  const applyDesktopUiSettingsRef = useLatestRef(applyDesktopUiSettings);
  const applyUiSettingsFallbackRef = useLatestRef(applyUiSettingsFallback);
  const toUserMessageRef = useLatestRef(toUserMessage);
  const onPairingCodeRotatedRef = useLatestRef(onPairingCodeRotated);
  const onConnectionRequestRef = useLatestRef(onConnectionRequest);
  const onConnectionEstablishedRef = useLatestRef(onConnectionEstablished);
  const onConnectionFailedRef = useLatestRef(onConnectionFailed);
  const onConnectionEndedRef = useLatestRef(onConnectionEnded);

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
          setStatusRef.current("online");
        }
        if (!connectedPeerRef.current) {
          const peer = await callCommand<ConnectedPeerPayload | null>("get_connected_peer");
          if (peer) {
            setConnectedPeerRef.current(mapConnectedPeerPayload(peer));
          }
        }
        return;
      }

      if (pairingStageRef.current === "idle") {
        if (connectedPeerRef.current) {
          setConnectedPeerRef.current(null);
          setLastMessageRef.current("当前连接已断开，请重新连接。");
        }
        setStatusRef.current("offline");
      }
    } catch {
      if (pairingStageRef.current === "idle") {
        setStatusRef.current("offline");
      }
    }
  }, [callCommand, pairingStageRef]);

  const refreshConnectionStatusRef = useLatestRef(refreshConnectionStatus);

  useEffect(() => {
    if (!tauriAvailable) {
      setPairingCodeRef.current("桌面端可用");
      setStatusRef.current("offline");
      setLastMessageRef.current("当前是浏览器预览模式，连接能力需在桌面应用中体验。");
      return;
    }

    let disposed = false;
    let eventCleanup: Array<() => void> = [];

    const setup = async () => {
      const listeners = await Promise.all([
        listen<LanDevicePayload[]>("lan-devices-changed", (event) => {
          setLanDevicesRef.current((previous) =>
            areLanDevicesEqual(previous, event.payload) ? previous : event.payload,
          );
        }),
        listen<ClipboardHistoryPayload[]>("clipboard-history-changed", (event) => {
          setClipsRef.current(mapClipboardHistory(event.payload));
        }),
        listen<ConnectionRequestPayload>("connection-request", (event) => {
          onConnectionRequestRef.current(event.payload);
        }),
        listen<ConnectionEstablishedPayload>("connection-established", (event) => {
          onConnectionEstablishedRef.current(event.payload);
        }),
        listen<ConnectionFailedPayload>("connection-failed", (event) => {
          onConnectionFailedRef.current(event.payload);
        }),
        listen<ConnectionEndedPayload>("connection-ended", (event) => {
          onConnectionEndedRef.current(event.payload);
        }),
        listen<PairingCodeRotatedPayload>("pairing-code-rotated", (event) => {
          setPairingCodeRef.current(event.payload.code);
          onPairingCodeRotatedRef.current?.(event.payload);
        }),
      ]);

      if (disposed) {
        listeners.forEach((unlisten) => unlisten());
        return;
      }

      eventCleanup = listeners;

      try {
        const [
          initialStatus,
          initialPairingCode,
          initialLanDevices,
          initialTrustedPeers,
          initialUiSettings,
          initialClipboardHistory,
          initialConnectedPeer,
          initialPendingConnectionRequest,
        ] = await Promise.all([
          callCommand<string>("get_status"),
          callCommand<string>("get_pairing_code"),
          callCommand<LanDevicePayload[]>("get_lan_devices"),
          callCommand<TrustedPeerPayload[]>("get_trusted_peers"),
          callCommand<UiSettingsPayload>("get_ui_settings"),
          callCommand<ClipboardHistoryPayload[]>("get_clipboard_history"),
          callCommand<ConnectedPeerPayload | null>("get_connected_peer"),
          callCommand<ConnectionRequestPayload | null>("get_pending_connection_request"),
        ]);

        if (disposed) {
          return;
        }

        setPairingCodeRef.current(initialPairingCode);
        setLanDevicesRef.current(initialLanDevices);
        setTrustedPeersRef.current(initialTrustedPeers);
        setClipsRef.current(mapClipboardHistory(initialClipboardHistory));
        applyDesktopUiSettingsRef.current(initialUiSettings);
        if (initialStatus === "connected") {
          setStatusRef.current("online");
          if (initialConnectedPeer) {
            setConnectedPeerRef.current(mapConnectedPeerPayload(initialConnectedPeer));
          }
          setLastMessageRef.current("已恢复现有连接，可以继续同步剪贴板。");
        } else {
          setStatusRef.current("offline");
          setLastMessageRef.current("正在监听设备列表与连接请求。");
        }

        if (initialPendingConnectionRequest) {
          onConnectionRequestRef.current(initialPendingConnectionRequest);
        }
      } catch (error) {
        if (!disposed) {
          setStatusRef.current("offline");
          applyUiSettingsFallbackRef.current();
          setLastMessageRef.current(toUserMessageRef.current(error, "连接桥接初始化失败，请稍后再试。"));
        }
      }
    };

    void setup();

    const timer = window.setInterval(() => {
      void refreshConnectionStatusRef.current();
    }, 5_000);

    return () => {
      disposed = true;
      window.clearInterval(timer);
      eventCleanup.forEach((unlisten) => unlisten());
    };
  }, [callCommand, tauriAvailable]);
}
