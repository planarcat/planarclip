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
  OutboundConnectionPendingPayload,
  OutboundConnectionSettledPayload,
  PairingCodeNeededPayload,
  PairingCodeRotatedPayload,
  PairingStage,
  ShellBootstrapPayload,
  ShellDeferredPayload,
  TrustedPeerPayload,
  UiSettingsPayload,
} from "../types";
import type { ClipboardSyncActivityPayload } from "./useTransferProgress";
import { mapClipboardHistory } from "../utils/clipboard";
import { areLanDevicesEqual, inferOs } from "../utils/device";
import { scheduleDeferred } from "../utils/scheduleDeferred";

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

function mapConnectedPeersPayload(payload: ConnectedPeerPayload[]): ConnectedPeer[] {
  return payload.map(mapConnectedPeerPayload);
}

function areConnectedPeersEqual(left: ConnectedPeer[], right: ConnectedPeer[]): boolean {
  if (left.length !== right.length) {
    return false;
  }

  return left.every((peer, index) => {
    const other = right[index];
    return peer.peerId === other.peerId && peer.name === other.name;
  });
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
  connectedPeers: ConnectedPeer[];
  pairingStageRef: React.MutableRefObject<PairingStage>;
  setStatus: (status: AppConnectionStatus) => void;
  setLastMessage: (message: string) => void;
  showNotice?: (message: string) => void;
  onSyncActivity?: (payload: ClipboardSyncActivityPayload) => void;
  setPairingCode: (pairingCode: string) => void;
  onPairingCodeRotated?: (payload: PairingCodeRotatedPayload) => void;
  setLanDevices: Dispatch<SetStateAction<LanDevicePayload[]>>;
  setTrustedPeers: (peers: TrustedPeerPayload[]) => void;
  setClips: (clips: ReturnType<typeof mapClipboardHistory>) => void;
  setConnectedPeers: Dispatch<SetStateAction<ConnectedPeer[]>>;
  applyDesktopUiSettings: (settings: UiSettingsPayload) => void;
  applyUiSettingsFallback: () => void;
  toUserMessage: (error: unknown, fallback: string, targetName?: string) => string;
  onConnectionRequest: (payload: ConnectionRequestPayload) => void;
  onConnectionEstablished: (payload: ConnectionEstablishedPayload) => void;
  onConnectionFailed: (payload: ConnectionFailedPayload) => void;
  onConnectionEnded: (payload: ConnectionEndedPayload) => void;
  onOutboundConnectionStarted: (payload: OutboundConnectionPendingPayload) => void;
  onOutboundConnectionPending: (payload: OutboundConnectionPendingPayload) => void;
  onOutboundConnectionSettled: (payload: OutboundConnectionSettledPayload) => void;
  onPairingCodeNeeded: (payload: PairingCodeNeededPayload) => void;
  onBackendConnectionSynced?: (peers: ConnectedPeer[]) => void;
  onShellDeferred?: (payload: ShellDeferredPayload) => void;
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
  connectedPeers,
  pairingStageRef,
  setStatus,
  setLastMessage,
  showNotice,
  onSyncActivity,
  setPairingCode,
  onPairingCodeRotated,
  setLanDevices,
  setTrustedPeers,
  setClips,
  setConnectedPeers,
  applyDesktopUiSettings,
  applyUiSettingsFallback,
  toUserMessage,
  onConnectionRequest,
  onConnectionEstablished,
  onConnectionFailed,
  onConnectionEnded,
  onOutboundConnectionStarted,
  onOutboundConnectionPending,
  onOutboundConnectionSettled,
  onPairingCodeNeeded,
  onBackendConnectionSynced,
  onShellDeferred,
}: UseConnectionBridgeOptions) {
  const statusRef = useRef(status);
  const connectedPeersRef = useRef(connectedPeers);

  const setLanDevicesRef = useLatestRef(setLanDevices);
  const setClipsRef = useLatestRef(setClips);
  const setPairingCodeRef = useLatestRef(setPairingCode);
  const setStatusRef = useLatestRef(setStatus);
  const setLastMessageRef = useLatestRef(setLastMessage);
  const showNoticeRef = useLatestRef(showNotice);
  const onSyncActivityRef = useLatestRef(onSyncActivity);
  const setTrustedPeersRef = useLatestRef(setTrustedPeers);
  const setConnectedPeersRef = useLatestRef(setConnectedPeers);
  const applyDesktopUiSettingsRef = useLatestRef(applyDesktopUiSettings);
  const applyUiSettingsFallbackRef = useLatestRef(applyUiSettingsFallback);
  const toUserMessageRef = useLatestRef(toUserMessage);
  const onPairingCodeRotatedRef = useLatestRef(onPairingCodeRotated);
  const onConnectionRequestRef = useLatestRef(onConnectionRequest);
  const onConnectionEstablishedRef = useLatestRef(onConnectionEstablished);
  const onConnectionFailedRef = useLatestRef(onConnectionFailed);
  const onConnectionEndedRef = useLatestRef(onConnectionEnded);
  const onOutboundConnectionStartedRef = useLatestRef(onOutboundConnectionStarted);
  const onOutboundConnectionPendingRef = useLatestRef(onOutboundConnectionPending);
  const onOutboundConnectionSettledRef = useLatestRef(onOutboundConnectionSettled);
  const onPairingCodeNeededRef = useLatestRef(onPairingCodeNeeded);
  const onBackendConnectionSyncedRef = useLatestRef(onBackendConnectionSynced);
  const onShellDeferredRef = useLatestRef(onShellDeferred);

  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  useEffect(() => {
    connectedPeersRef.current = connectedPeers;
  }, [connectedPeers]);

  const refreshConnectionStatus = useCallback(async () => {
    try {
      const connectionStatus = await callCommand<string>("get_status");
      const isConnected = connectionStatus === "connected";

      if (isConnected) {
        if (statusRef.current !== "online") {
          setStatusRef.current("online");
        }

        const peers = await callCommand<ConnectedPeerPayload[]>("get_connected_peers");
        const mappedPeers = mapConnectedPeersPayload(peers);
        if (!areConnectedPeersEqual(connectedPeersRef.current, mappedPeers)) {
          setConnectedPeersRef.current(mappedPeers);
        }
        onBackendConnectionSyncedRef.current?.(mappedPeers);
        return;
      }

      if (pairingStageRef.current === "idle") {
        if (connectedPeersRef.current.length > 0) {
          setConnectedPeersRef.current([]);
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
        listen<OutboundConnectionPendingPayload>("outbound-connection-started", (event) => {
          onOutboundConnectionStartedRef.current(event.payload);
        }),
        listen<OutboundConnectionPendingPayload>("outbound-connection-pending", (event) => {
          onOutboundConnectionPendingRef.current(event.payload);
        }),
        listen<OutboundConnectionSettledPayload>("outbound-connection-settled", (event) => {
          onOutboundConnectionSettledRef.current(event.payload);
        }),
        listen<PairingCodeNeededPayload>("pairing-code-needed", (event) => {
          onPairingCodeNeededRef.current(event.payload);
        }),
        listen<PairingCodeRotatedPayload>("pairing-code-rotated", (event) => {
          setPairingCodeRef.current(event.payload.code);
          onPairingCodeRotatedRef.current?.(event.payload);
        }),
        listen<ClipboardSyncActivityPayload>("clipboard-sync-activity", (event) => {
          const payload = event.payload;
          if (!payload.message) {
            return;
          }

          setLastMessageRef.current(payload.message);
          onSyncActivityRef.current?.(payload);

          if (payload.kind === "notice") {
            showNoticeRef.current?.(payload.message);
          }
        }),
      ]);

      if (disposed) {
        listeners.forEach((unlisten) => unlisten());
        return;
      }

      eventCleanup = listeners;

      try {
        const bootstrap = await callCommand<ShellBootstrapPayload>("get_shell_bootstrap");

        if (disposed) {
          return;
        }

        setPairingCodeRef.current(bootstrap.pairing_code);
        applyDesktopUiSettingsRef.current(bootstrap.ui_settings);

        if (bootstrap.status === "connected") {
          setStatusRef.current("online");
          if (bootstrap.connected_peers.length > 0) {
            setConnectedPeersRef.current(mapConnectedPeersPayload(bootstrap.connected_peers));
          }
          setLastMessageRef.current("已恢复现有连接，可以继续同步剪贴板。");
        } else {
          setStatusRef.current("offline");
          setLastMessageRef.current("正在监听设备列表与连接请求。");
        }

        if (bootstrap.pending_connection_request) {
          onConnectionRequestRef.current(bootstrap.pending_connection_request);
        }

        scheduleDeferred(() => {
          if (disposed) {
            return;
          }
          void (async () => {
            try {
              const deferred = await callCommand<ShellDeferredPayload>("get_shell_deferred");
              if (disposed) {
                return;
              }
              setLanDevicesRef.current(deferred.lan_devices);
              setTrustedPeersRef.current(deferred.trusted_peers);
              setClipsRef.current(mapClipboardHistory(deferred.clipboard_history));
              onShellDeferredRef.current?.(deferred);
            } catch (error) {
              if (!disposed) {
                setLastMessageRef.current(
                  toUserMessageRef.current(error, "部分数据加载较慢，请稍后刷新。"),
                );
              }
            }
          })();
        });
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
