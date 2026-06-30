import { useCallback, useEffect, useRef, type Dispatch, type SetStateAction } from "react";
import { MAX_CONNECTIONS } from "../constants/connection";
import type {
  AppConnectionStatus,
  CommandExecutor,
  ConnectedPeer,
  ConnectionEndedPayload,
  ConnectionEstablishedPayload,
  ConnectionFailedPayload,
  ConnectionRequestPayload,
  Device,
  OutboundConnectionPendingPayload,
  PairingCodeNeededPayload,
  PairingStage,
} from "../types";
import { formatDeviceAddress, inferOs } from "../utils/device";
import {
  MSG_CONNECTION_LIMIT,
  MSG_INVALID_PAIRING_CODE,
  MSG_PAIRING_CODE_REFRESHED,
  MSG_ENTER_PEER_PAIRING_CODE,
  MSG_WAIT_FOR_PEER_PAIRING_CODE,
  MSG_PEER_CANCELLED,
  MSG_PEER_REJECTED,
  MSG_PEER_RESPONSE_TIMEOUT,
  MSG_SELF_CANCELLED_INBOUND,
  MSG_SELF_CANCELLED_OUTBOUND,
  MSG_SELF_INCOMING_TIMEOUT,
  connectionUnavailableMessage,
  isConnectionRejected,
  isConnectionTimeout,
  isInvalidPairingCode,
  isPeerCancelled,
  isPeerOffline,
  normalizeUserMessage,
  connectionEndedMessage,
} from "../utils/message";
import { formatTime } from "../utils/time";

function deviceFromOutboundPeer(payload: {
  peer_id: string;
  peer_name: string;
  peer_ip: string;
  peer_port: number;
}): Device {
  return {
    id: `outbound:${payload.peer_id}`,
    name: payload.peer_name,
    os: inferOs(payload.peer_name),
    host: payload.peer_ip,
    port: payload.peer_port,
    peerId: payload.peer_id,
    address: formatDeviceAddress(payload.peer_ip, payload.peer_port),
    status: "idle",
    source: "trusted",
    isTrusted: true,
    discoveredOnLan: true,
  };
}

type UsePairingFlowOptions = {
  callCommand: CommandExecutor;
  status: AppConnectionStatus;
  connectedCount: number;
  pairingInput: string;
  pairingStage: PairingStage;
  pairingTarget: Device | null;
  incomingRequest: ConnectionRequestPayload | null;
  setStatus: (status: AppConnectionStatus) => void;
  setLastMessage: (message: string) => void;
  setConnectedPeers: Dispatch<SetStateAction<ConnectedPeer[]>>;
  setShowPairing: (show: boolean) => void;
  setPairingInput: (value: string) => void;
  setPairingStage: (stage: PairingStage) => void;
  setPairingTarget: (target: Device | null) => void;
  setPairingHelperText: (message: string) => void;
  setPairingError: (message: string | null) => void;
  setPairingRotationHint: (message: string | null) => void;
  setPairingCode: (code: string) => void;
  setIncomingRequest: (payload: ConnectionRequestPayload | null) => void;
  refreshLanDevices: () => Promise<void>;
  showNotice: (message: string) => void;
};

const OUTBOUND_LOCK_STAGES: PairingStage[] = ["requesting_device", "awaiting_code", "submitting_code"];

const INBOUND_CANCEL_STAGES: PairingStage[] = ["incoming_pairing", "incoming_accepting"];

function isOutboundLockedStage(stage: PairingStage) {
  return OUTBOUND_LOCK_STAGES.includes(stage);
}

function isInboundCancelStage(stage: PairingStage) {
  return INBOUND_CANCEL_STAGES.includes(stage);
}

function resolveAppStatus(connectedCount: number, connecting: boolean): AppConnectionStatus {
  if (connecting) {
    return "connecting";
  }
  return connectedCount > 0 ? "online" : "offline";
}

function upsertConnectedPeer(peers: ConnectedPeer[], peer: ConnectedPeer) {
  const without = peers.filter((item) => item.peerId !== peer.peerId);
  return [...without, peer];
}

/**
 * 管理配对弹层状态、手动配对、局域网连接与事件驱动的连接结果回写。
 */
export function usePairingFlow({
  callCommand,
  connectedCount,
  pairingInput,
  pairingStage,
  pairingTarget,
  incomingRequest,
  setStatus,
  setLastMessage,
  setConnectedPeers,
  setShowPairing,
  setPairingInput,
  setPairingStage,
  setPairingTarget,
  setPairingHelperText,
  setPairingError,
  setPairingRotationHint,
  setPairingCode,
  setIncomingRequest,
  refreshLanDevices,
  showNotice,
}: UsePairingFlowOptions) {
  const pairingStageRef = useRef(pairingStage);
  const pairingTargetRef = useRef<Device | null>(pairingTarget);
  const lastTerminalNoticeAtRef = useRef(0);
  const outboundCancelledRef = useRef(false);

  useEffect(() => {
    pairingStageRef.current = pairingStage;
  }, [pairingStage]);

  useEffect(() => {
    pairingTargetRef.current = pairingTarget;
  }, [pairingTarget]);

  const resetPairingFlow = useCallback(
    (closeModal = false) => {
      setPairingInput("");
      setPairingStage("idle");
      setPairingTarget(null);
      setPairingHelperText("请先从列表中选择要连接的设备。");
      setPairingError(null);
      setPairingRotationHint(null);
      setIncomingRequest(null);
      if (closeModal) {
        setShowPairing(false);
      }
    },
    [
      setIncomingRequest,
      setPairingError,
      setPairingHelperText,
      setPairingInput,
      setPairingRotationHint,
      setPairingStage,
      setPairingTarget,
      setShowPairing,
    ],
  );

  const showTerminalNotice = useCallback(
    (message: string) => {
      const now = Date.now();
      if (now - lastTerminalNoticeAtRef.current < 500) {
        return;
      }
      lastTerminalNoticeAtRef.current = now;
      showNotice(message);
      setLastMessage(message);
    },
    [setLastMessage, showNotice],
  );

  const handleTerminalConnectionFailure = useCallback(
    (message: string) => {
      showTerminalNotice(message);
      setStatus(resolveAppStatus(connectedCount, false));
      resetPairingFlow(true);
      void refreshLanDevices();
    },
    [connectedCount, refreshLanDevices, resetPairingFlow, setStatus, showTerminalNotice],
  );

  const abortOutboundConnection = useCallback(async () => {
    outboundCancelledRef.current = true;
    try {
      await callCommand("abort_outbound_connection");
    } catch {
    }
  }, [callCommand]);

  const endPairingSession = useCallback(async () => {
    try {
      await callCommand("end_pairing_session");
    } catch {
    }
  }, [callCommand]);

  const refreshPairingCode = useCallback(
    async (options?: { showHint?: boolean; clearInput?: boolean }) => {
      const showHint = options?.showHint ?? false;
      const clearInput = options?.clearInput ?? false;

      if (clearInput) {
        setPairingInput("");
      }
      if (showHint) {
        setPairingError(null);
      }

      try {
        const code = await callCommand<string>("rotate_pairing_code");
        setPairingCode(code);
        if (showHint) {
          setPairingRotationHint(MSG_PAIRING_CODE_REFRESHED);
        }
        return code;
      } catch {
        try {
          const code = await callCommand<string>("get_pairing_code");
          setPairingCode(code);
          if (showHint) {
            setPairingRotationHint(MSG_PAIRING_CODE_REFRESHED);
          }
          return code;
        } catch {
          return null;
        }
      }
    },
    [callCommand, setPairingCode, setPairingError, setPairingInput, setPairingRotationHint],
  );

  const openPairingModal = useCallback(() => {
    if (connectedCount >= MAX_CONNECTIONS) {
      showTerminalNotice(MSG_CONNECTION_LIMIT);
      return;
    }
    setPairingTarget(null);
    setPairingStage("idle");
    setPairingError(null);
    setPairingRotationHint(null);
    setPairingHelperText("请先从列表中选择要连接的设备。");
    setShowPairing(true);
    void refreshPairingCode();
  }, [
    connectedCount,
    refreshPairingCode,
    setPairingError,
    setPairingHelperText,
    setPairingRotationHint,
    setPairingStage,
    setPairingTarget,
    setShowPairing,
    showTerminalNotice,
  ]);

  const cancelInboundConnection = useCallback(async () => {
    try {
      await callCommand("reject_connection");
    } catch {
    }
    handleTerminalConnectionFailure(MSG_SELF_CANCELLED_INBOUND);
  }, [callCommand, handleTerminalConnectionFailure]);

  const closePairingModal = useCallback(async () => {
    const stage = pairingStageRef.current;

    if (isInboundCancelStage(stage)) {
      await cancelInboundConnection();
      return;
    }

    if (isOutboundLockedStage(stage)) {
      await abortOutboundConnection();
      handleTerminalConnectionFailure(MSG_SELF_CANCELLED_OUTBOUND);
      return;
    }

    void endPairingSession();
    resetPairingFlow(true);
  }, [
    abortOutboundConnection,
    cancelInboundConnection,
    endPairingSession,
    handleTerminalConnectionFailure,
    resetPairingFlow,
  ]);

  const handleRotatePairingCode = useCallback(async () => {
    const stage = pairingStageRef.current;
    if (stage !== "awaiting_code" && stage !== "incoming_pairing") {
      return;
    }

    await refreshPairingCode({ showHint: true, clearInput: true });
  }, [refreshPairingCode]);

  const executeConnectLan = useCallback(
    async (device: Device) => {
      if (!device.host || !device.port) {
        setPairingError("当前设备缺少连接地址，请等待下一轮发现结果。");
        return;
      }

      if (connectedCount >= MAX_CONNECTIONS) {
        showTerminalNotice(MSG_CONNECTION_LIMIT);
        return;
      }

      if (isOutboundLockedStage(pairingStageRef.current)) {
        return;
      }

      outboundCancelledRef.current = false;
      setShowPairing(false);
      setPairingTarget(device);
      setPairingStage("requesting_device");
      setPairingError(null);
      setPairingRotationHint(null);
      const helperMessage = `正在尝试连接 ${device.name}…`;
      setPairingHelperText(helperMessage);
      setStatus("connecting");
      setLastMessage(helperMessage);

      try {
        const result = await callCommand<string>("connect_lan", {
          ip: device.host,
          port: device.port,
          peerId: device.peerId,
        });

        if (outboundCancelledRef.current) {
          return;
        }

        if (result === "awaiting_code") {
          setShowPairing(true);
          setPairingStage("awaiting_code");
          setPairingHelperText(MSG_ENTER_PEER_PAIRING_CODE);
          setLastMessage(MSG_ENTER_PEER_PAIRING_CODE);
          setPairingInput("");
          await refreshPairingCode();
          return;
        }

        setConnectedPeers((previous) =>
          upsertConnectedPeer(previous, {
            name: device.name,
            peerId: device.peerId,
            address: device.address,
            os: device.os,
            source: "lan",
          }),
        );
        setStatus("online");
        setLastMessage(`已与 ${device.name} 建立连接，现在可以开始同步剪贴板了 — ${formatTime()}`);
        resetPairingFlow(true);
      } catch (error) {
        if (outboundCancelledRef.current) {
          return;
        }
        const message = normalizeUserMessage(error, connectionUnavailableMessage(device.name), device.name);
        if (isConnectionRejected(error)) {
          handleTerminalConnectionFailure(message);
          return;
        }
        setShowPairing(true);
        setPairingStage("error");
        setPairingError(message);
        setPairingHelperText(message);
        setStatus(resolveAppStatus(connectedCount, false));
        setLastMessage(message);
      }
    },
    [
      callCommand,
      connectedCount,
      handleTerminalConnectionFailure,
      resetPairingFlow,
      refreshPairingCode,
      setConnectedPeers,
      setLastMessage,
      setPairingError,
      setPairingHelperText,
      setPairingInput,
      setPairingRotationHint,
      setPairingStage,
      setPairingTarget,
      setShowPairing,
      setStatus,
      showTerminalNotice,
    ],
  );

  const handleConnectLan = useCallback(
    async (device: Device) => {
      if (device.status === "connected") {
        return;
      }

      await executeConnectLan(device);
    },
    [executeConnectLan],
  );

  const switchPairingTarget = useCallback(
    async (device: Device) => {
      if (pairingTargetRef.current?.id === device.id) {
        return;
      }

      const stage = pairingStageRef.current;
      if (stage === "submitting_code") {
        return;
      }

      if (stage === "incoming_pairing") {
        return;
      }

      if (isOutboundLockedStage(stage)) {
        outboundCancelledRef.current = true;
        await abortOutboundConnection();
        outboundCancelledRef.current = false;
      } else if (stage === "idle") {
        await refreshPairingCode();
      }

      setPairingInput("");
      setPairingStage("idle");
      setPairingError(null);
      setPairingRotationHint(null);
      setPairingTarget(device);
      await handleConnectLan(device);
    },
    [
      abortOutboundConnection,
      handleConnectLan,
      refreshPairingCode,
      setPairingError,
      setPairingInput,
      setPairingRotationHint,
      setPairingStage,
      setPairingTarget,
    ],
  );

  const handleSubmitPairingCode = useCallback(async () => {
    if (pairingInput.length !== 6) {
      setPairingError("请输入 6 位数字配对码。");
      return;
    }

    const inboundPairing = pairingStageRef.current === "incoming_pairing";
    setPairingStage("submitting_code");
    setPairingError(null);
    setPairingRotationHint(null);
    setStatus("connecting");

    try {
      const command = inboundPairing ? "submit_responder_pairing_code" : "submit_pairing_code";
      const result = await callCommand<string>(command, { code: pairingInput });
      if (inboundPairing && result === "verified") {
        setPairingStage("incoming_pairing");
        setPairingHelperText(MSG_WAIT_FOR_PEER_PAIRING_CODE);
        setLastMessage("已验证对方配对码，正在完成连接…");
      }
    } catch (error) {
      if (outboundCancelledRef.current) {
        return;
      }
      if (isConnectionRejected(error)) {
        handleTerminalConnectionFailure(normalizeUserMessage(error, MSG_PEER_REJECTED));
        return;
      }
      if (isInvalidPairingCode(error)) {
        setPairingStage(inboundPairing ? "incoming_pairing" : "awaiting_code");
        setPairingError(MSG_INVALID_PAIRING_CODE);
        setPairingHelperText(MSG_INVALID_PAIRING_CODE);
        setStatus("connecting");
        return;
      }
      const message = normalizeUserMessage(
        error,
        connectionUnavailableMessage(pairingTargetRef.current?.name),
        pairingTargetRef.current?.name,
      );
      handleTerminalConnectionFailure(message);
    }
  }, [callCommand, handleTerminalConnectionFailure, pairingInput, setPairingError, setPairingHelperText, setPairingRotationHint, setPairingStage, setStatus]);

  const dismissIncomingRequest = useCallback(
    async (message: string, mode: "reject" | "timeout" = "reject") => {
      try {
        if (mode === "timeout") {
          await callCommand("timeout_incoming_connection");
        } else {
          await callCommand("reject_connection");
        }
      } catch {
      }
      showTerminalNotice(message);
      resetPairingFlow(true);
      setStatus(resolveAppStatus(connectedCount, false));
    },
    [callCommand, connectedCount, resetPairingFlow, setStatus, showTerminalNotice],
  );

  const handleRejectIncoming = useCallback(async () => {
    await dismissIncomingRequest("已拒绝这次连接请求。", "reject");
  }, [dismissIncomingRequest]);

  const handleIncomingResponseTimeout = useCallback(async () => {
    await dismissIncomingRequest(MSG_SELF_INCOMING_TIMEOUT, "timeout");
  }, [dismissIncomingRequest]);

  const handleAcceptIncoming = useCallback(async () => {
    if (!incomingRequest) {
      return;
    }

    const peerName = incomingRequest.device_name;
    setPairingError(null);
    setPairingRotationHint(null);
    setStatus("connecting");
    setPairingStage("incoming_accepting");
    setLastMessage(`正在允许 ${peerName} 连接…`);

    try {
      await callCommand("accept_connection");

      if (incomingRequest.requires_pairing) {
        setShowPairing(true);
        setPairingStage("incoming_pairing");
        setPairingHelperText(MSG_ENTER_PEER_PAIRING_CODE);
        setLastMessage(MSG_ENTER_PEER_PAIRING_CODE);
        await refreshPairingCode();
      }
    } catch (error) {
      const message = normalizeUserMessage(error, MSG_PEER_CANCELLED, peerName);
      if (isConnectionRejected(error) || message === MSG_PEER_CANCELLED) {
        handleTerminalConnectionFailure(message);
        return;
      }
      setPairingStage("error");
      setPairingError(message);
      setPairingHelperText(message);
      setStatus(resolveAppStatus(connectedCount, false));
      setLastMessage(message);
    }
  }, [
    callCommand,
    connectedCount,
    handleTerminalConnectionFailure,
    incomingRequest,
    refreshPairingCode,
    setLastMessage,
    setPairingCode,
    setPairingError,
    setPairingHelperText,
    setPairingRotationHint,
    setPairingStage,
    setShowPairing,
    setStatus,
  ]);

  const handleDisconnect = useCallback(async (device?: Device) => {
    try {
      if (device?.peerId) {
        await callCommand("disconnect_peer", { peerId: device.peerId });
        setConnectedPeers((previous) => previous.filter((peer) => peer.peerId !== device.peerId));
        setLastMessage(`已断开与 ${device.name} 的连接。`);
        setStatus(resolveAppStatus(Math.max(0, connectedCount - 1), false));
      } else {
        await callCommand("disconnect");
        setConnectedPeers([]);
        setStatus("offline");
        setLastMessage("已断开所有连接。");
        resetPairingFlow(true);
      }
      void refreshLanDevices();
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "断开连接时出了点问题，请稍后再试。"));
    }
  }, [callCommand, connectedCount, refreshLanDevices, resetPairingFlow, setConnectedPeers, setLastMessage, setStatus]);

  const handleConnectionRequest = useCallback(
    (payload: ConnectionRequestPayload) => {
      setIncomingRequest(payload);
      setPairingError(null);
      setPairingRotationHint(null);
      setStatus("connecting");

      if (payload.requires_pairing) {
        setPairingStage("incoming_request");
        setPairingHelperText(`${payload.device_name} 是陌生设备，请先确认是否允许配对。`);
        setLastMessage(`${payload.device_name} 正在请求连接，请在确认窗口中选择是否允许。`);
        return;
      }

      setPairingStage("incoming_request");
      setPairingHelperText(`${payload.device_name} 正在请求连接，请确认是否允许。`);
      setLastMessage(`${payload.device_name} 正在请求连接，请在确认窗口中选择是否允许。`);
    },
    [setIncomingRequest, setLastMessage, setPairingError, setPairingHelperText, setPairingRotationHint, setPairingStage, setStatus],
  );

  const handleConnectionEstablished = useCallback(
    (payload: ConnectionEstablishedPayload) => {
      if (outboundCancelledRef.current) {
        if (payload.peer_id) {
          void callCommand("disconnect_peer", { peerId: payload.peer_id }).catch(() => {});
        }
        return;
      }

      const targetName = pairingTargetRef.current?.name;
      setConnectedPeers((previous) =>
        upsertConnectedPeer(previous, {
          name: payload.peer_name || "已连接设备",
          peerId: payload.peer_id,
          address: targetName ? `${targetName} · 局域网直连` : "局域网直连",
          os: inferOs(payload.peer_name || "已连接设备"),
          source: "lan",
        }),
      );
      setStatus("online");
      setLastMessage(
        payload.is_reconnect
          ? `${payload.peer_name} 已连接`
          : `已与 ${payload.peer_name} 建立连接，现在可以开始同步剪贴板了 — ${formatTime()}`,
      );
      resetPairingFlow(true);
    },
    [callCommand, resetPairingFlow, setConnectedPeers, setLastMessage, setStatus],
  );

  const handleConnectionFailed = useCallback(
    (payload: ConnectionFailedPayload) => {
      if (outboundCancelledRef.current) {
        return;
      }

      const inboundWaitStage =
        pairingStageRef.current === "incoming_request" ||
        pairingStageRef.current === "incoming_accepting";

      const inboundPairingStage =
        pairingStageRef.current === "incoming_pairing" ||
        pairingStageRef.current === "incoming_accepting";

      const outboundPairingStage =
        pairingStageRef.current === "requesting_device" ||
        pairingStageRef.current === "awaiting_code" ||
        pairingStageRef.current === "submitting_code";

      const pairingInProgress = inboundPairingStage || outboundPairingStage;

      if (pairingInProgress && (isPeerCancelled(payload) || isConnectionRejected(payload))) {
        handleTerminalConnectionFailure(
          normalizeUserMessage(payload, MSG_PEER_CANCELLED, pairingTargetRef.current?.name ?? incomingRequest?.device_name),
        );
        return;
      }

      if (pairingInProgress && isPeerOffline(payload)) {
        handleTerminalConnectionFailure(MSG_PEER_CANCELLED);
        return;
      }

      const message = normalizeUserMessage(
        payload,
        inboundWaitStage ? MSG_PEER_CANCELLED : connectionUnavailableMessage(pairingTargetRef.current?.name),
        pairingTargetRef.current?.name ?? incomingRequest?.device_name,
      );

      if (isConnectionTimeout(payload)) {
        handleTerminalConnectionFailure(MSG_PEER_RESPONSE_TIMEOUT);
        return;
      }

      if (isConnectionRejected(payload)) {
        handleTerminalConnectionFailure(message);
        return;
      }

      if (inboundWaitStage && isPeerOffline(payload)) {
        handleTerminalConnectionFailure(MSG_PEER_CANCELLED);
        return;
      }

      if (isInvalidPairingCode(payload) && pairingStageRef.current === "awaiting_code") {
        setPairingStage("awaiting_code");
        setPairingError(MSG_INVALID_PAIRING_CODE);
        setPairingHelperText(MSG_INVALID_PAIRING_CODE);
        setStatus("connecting");
        setLastMessage(MSG_INVALID_PAIRING_CODE);
        return;
      }

      if (
        pairingStageRef.current === "incoming_request" ||
        pairingStageRef.current === "incoming_pairing" ||
        pairingStageRef.current === "incoming_accepting"
      ) {
        resetPairingFlow(false);
      }
      setPairingStage("error");
      setPairingError(message);
      setPairingHelperText(message);
      setStatus(resolveAppStatus(connectedCount, false));
      setLastMessage(message);
    },
    [connectedCount, handleTerminalConnectionFailure, incomingRequest, resetPairingFlow, setLastMessage, setPairingError, setPairingHelperText, setPairingStage, setStatus],
  );

  const handleConnectionEnded = useCallback(
    async (payload: ConnectionEndedPayload) => {
      const message = connectionEndedMessage(payload);
      setConnectedPeers((previous) => {
        const next = previous.filter((peer) => peer.peerId !== payload.peer_id);
        setStatus(resolveAppStatus(next.length, false));
        return next;
      });
      showTerminalNotice(message);
      if (pairingStageRef.current !== "idle") {
        resetPairingFlow(true);
      }
      void refreshLanDevices();
    },
    [refreshLanDevices, resetPairingFlow, setConnectedPeers, setStatus, showTerminalNotice],
  );

  const beginOutboundAttemptUi = useCallback(
    (device: Device) => {
      if (outboundCancelledRef.current || isOutboundLockedStage(pairingStageRef.current)) {
        return;
      }

      outboundCancelledRef.current = false;
      setShowPairing(false);
      setPairingTarget(device);
      setPairingStage("requesting_device");
      setPairingError(null);
      setPairingRotationHint(null);
      const helperMessage = `正在尝试连接 ${device.name}…`;
      setPairingHelperText(helperMessage);
      setStatus("connecting");
      setLastMessage(helperMessage);
    },
    [
      setLastMessage,
      setPairingError,
      setPairingHelperText,
      setPairingRotationHint,
      setPairingStage,
      setPairingTarget,
      setShowPairing,
      setStatus,
    ],
  );

  const handleOutboundConnectionStarted = useCallback(
    (payload: OutboundConnectionPendingPayload) => {
      beginOutboundAttemptUi(deviceFromOutboundPeer(payload));
    },
    [beginOutboundAttemptUi],
  );

  const handleOutboundConnectionPending = useCallback(
    (payload: OutboundConnectionPendingPayload) => {
      beginOutboundAttemptUi(deviceFromOutboundPeer(payload));
    },
    [beginOutboundAttemptUi],
  );

  const handlePairingCodeNeeded = useCallback(
    async (payload: PairingCodeNeededPayload) => {
      if (outboundCancelledRef.current) {
        return;
      }

      const device = payload.peer_id
        ? deviceFromOutboundPeer({
            peer_id: payload.peer_id,
            peer_name: payload.peer_name ?? "对方设备",
            peer_ip: payload.peer_ip,
            peer_port: payload.peer_port ?? 0,
          })
        : null;

      if (device && device.port === 0) {
        device.address = payload.peer_ip;
      }

      setShowPairing(true);
      if (device) {
        setPairingTarget(device);
      }
      setPairingStage("awaiting_code");
      setPairingError(null);
      setPairingRotationHint(null);
      setPairingHelperText(MSG_ENTER_PEER_PAIRING_CODE);
      setPairingInput("");
      setStatus("connecting");
      setLastMessage(MSG_ENTER_PEER_PAIRING_CODE);

      await refreshPairingCode();
    },
    [
      refreshPairingCode,
      setLastMessage,
      setPairingError,
      setPairingHelperText,
      setPairingInput,
      setPairingRotationHint,
      setPairingStage,
      setPairingTarget,
      setShowPairing,
      setStatus,
    ],
  );

  const connectionLocked = isOutboundLockedStage(pairingStage);

  return {
    openPairingModal,
    closePairingModal,
    handleConnectLan,
    switchPairingTarget,
    handleSubmitPairingCode,
    handleRotatePairingCode,
    handleAcceptIncoming,
    handleRejectIncoming,
    handleIncomingResponseTimeout,
    handleDisconnect,
    handleConnectionRequest,
    handleConnectionEstablished,
    handleConnectionFailed,
    handleConnectionEnded,
    handleOutboundConnectionStarted,
    handleOutboundConnectionPending,
    handlePairingCodeNeeded,
    pairingStageRef,
    connectionLocked,
  };
}
