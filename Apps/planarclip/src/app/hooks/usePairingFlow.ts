import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
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
  LanDevicePayload,
  OutboundConnectionPendingPayload,
  PairingCodeNeededPayload,
  PairingStage,
} from "../types";
import { formatDeviceAddress, inferOs } from "../utils/device";
import {
  MSG_CONNECTION_LIMIT,
  MSG_INVALID_PAIRING_CODE,
  MSG_PAIRING_CODE_REFRESHED,
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
  isPeerOffline,
  normalizeUserMessage,
  peerOfflineMessage,
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
  connectedPeer: ConnectedPeer | null;
  pairingInput: string;
  pairingStage: PairingStage;
  pairingTarget: Device | null;
  incomingRequest: ConnectionRequestPayload | null;
  setStatus: (status: AppConnectionStatus) => void;
  setLastMessage: (message: string) => void;
  setConnectedPeer: (peer: ConnectedPeer | null) => void;
  setShowPairing: (show: boolean) => void;
  setPairingInput: (value: string) => void;
  setPairingStage: (stage: PairingStage) => void;
  setPairingTarget: (target: Device | null) => void;
  setPairingHelperText: (message: string) => void;
  setPairingError: (message: string | null) => void;
  setPairingRotationHint: (message: string | null) => void;
  setPairingCode: (code: string) => void;
  setIncomingRequest: (payload: ConnectionRequestPayload | null) => void;
  setLanDevices: Dispatch<SetStateAction<LanDevicePayload[]>>;
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

/**
 * 管理配对弹层状态、手动配对、局域网连接与事件驱动的连接结果回写。
 */
export function usePairingFlow({
  callCommand,
  connectedCount,
  connectedPeer,
  pairingInput,
  pairingStage,
  pairingTarget,
  incomingRequest,
  setStatus,
  setLastMessage,
  setConnectedPeer,
  setShowPairing,
  setPairingInput,
  setPairingStage,
  setPairingTarget,
  setPairingHelperText,
  setPairingError,
  setPairingRotationHint,
  setPairingCode,
  setIncomingRequest,
  setLanDevices,
  showNotice,
}: UsePairingFlowOptions) {
  const pairingStageRef = useRef(pairingStage);
  const pairingTargetRef = useRef<Device | null>(pairingTarget);
  const lastTerminalNoticeAtRef = useRef(0);
  const outboundCancelledRef = useRef(false);
  const [switchConnectionTarget, setSwitchConnectionTarget] = useState<Device | null>(null);

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
    },
    [connectedCount, resetPairingFlow, setStatus, showTerminalNotice],
  );

  const abortOutboundConnection = useCallback(async () => {
    outboundCancelledRef.current = true;
    try {
      await callCommand("disconnect");
    } catch {
    }
  }, [callCommand]);

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
  }, [connectedCount, setPairingError, setPairingHelperText, setPairingRotationHint, setPairingStage, setPairingTarget, setShowPairing, showTerminalNotice]);

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

    resetPairingFlow(true);
  }, [
    abortOutboundConnection,
    cancelInboundConnection,
    handleTerminalConnectionFailure,
    resetPairingFlow,
  ]);

  const handleRotatePairingCode = useCallback(async () => {
    if (pairingStageRef.current === "awaiting_code") {
      setPairingRotationHint(MSG_PAIRING_CODE_REFRESHED);
      setPairingError(null);
      setPairingInput("");
      return;
    }

    try {
      const code = await callCommand<string>("rotate_pairing_code");
      setPairingCode(code);
      setPairingRotationHint(MSG_PAIRING_CODE_REFRESHED);
      setPairingError(null);
    } catch {
      try {
        const code = await callCommand<string>("get_pairing_code");
        setPairingCode(code);
        setPairingRotationHint(MSG_PAIRING_CODE_REFRESHED);
      } catch {
      }
    }
  }, [callCommand, setPairingCode, setPairingError, setPairingInput, setPairingRotationHint]);

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
      setShowPairing(true);
      setPairingTarget(device);
      setPairingStage("requesting_device");
      setPairingError(null);
      setPairingRotationHint(null);
      const helperMessage = `正在等待 ${device.name} 回应…`;
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
          setPairingHelperText("请输入对方设备上显示的 6 位配对码。");
          setLastMessage("请输入对方设备上显示的 6 位配对码。");
          setPairingInput("");
          try {
            const code = await callCommand<string>("get_pairing_code");
            setPairingCode(code);
          } catch {
          }
          return;
        }

        setConnectedPeer({
          name: device.name,
          peerId: device.peerId,
          address: device.address,
          os: device.os,
          source: "lan",
        });
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
      setConnectedPeer,
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
        await executeConnectLan(device);
        return;
      }

      if (connectedCount > 0) {
        setSwitchConnectionTarget(device);
        return;
      }

      await executeConnectLan(device);
    },
    [connectedCount, executeConnectLan],
  );

  const confirmSwitchConnection = useCallback(async () => {
    const target = switchConnectionTarget;
    if (!target) {
      return;
    }

    setSwitchConnectionTarget(null);
    try {
      await callCommand("disconnect");
      setConnectedPeer(null);
    } catch {
      setLastMessage("断开当前连接时出了点问题，请稍后再试。");
      return;
    }

    await executeConnectLan(target);
  }, [callCommand, executeConnectLan, setConnectedPeer, setLastMessage, switchConnectionTarget]);

  const cancelSwitchConnection = useCallback(() => {
    setSwitchConnectionTarget(null);
  }, []);

  const switchPairingTarget = useCallback(
    async (device: Device) => {
      if (isOutboundLockedStage(pairingStageRef.current)) {
        return;
      }
      setPairingInput("");
      setPairingStage("idle");
      setPairingError(null);
      setPairingRotationHint(null);
      setPairingTarget(device);
      await handleConnectLan(device);
    },
    [handleConnectLan, setPairingError, setPairingInput, setPairingRotationHint, setPairingStage, setPairingTarget],
  );

  const handleSubmitPairingCode = useCallback(async () => {
    if (pairingInput.length !== 6) {
      setPairingError("请输入 6 位数字配对码。");
      return;
    }

    setPairingStage("submitting_code");
    setPairingError(null);
    setPairingRotationHint(null);
    setStatus("connecting");

    try {
      await callCommand<string>("submit_pairing_code", { code: pairingInput });
    } catch (error) {
      if (outboundCancelledRef.current) {
        return;
      }
      if (isConnectionRejected(error)) {
        handleTerminalConnectionFailure(normalizeUserMessage(error, MSG_PEER_REJECTED));
        return;
      }
      if (isInvalidPairingCode(error)) {
        setPairingStage("awaiting_code");
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
        setPairingHelperText("请输入对方设备上显示的 6 位配对码。");
        setLastMessage("请输入对方设备上显示的 6 位配对码。");
        try {
          const code = await callCommand<string>("get_pairing_code");
          setPairingCode(code);
        } catch {
        }
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
    setLastMessage,
    setPairingCode,
    setPairingError,
    setPairingHelperText,
    setPairingRotationHint,
    setPairingStage,
    setShowPairing,
    setStatus,
  ]);

  const handleDisconnect = useCallback(async () => {
    try {
      await callCommand("disconnect");
      setConnectedPeer(null);
      setStatus("offline");
      setLastMessage("已断开当前连接。");
      resetPairingFlow(true);
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "断开连接时出了点问题，请稍后再试。"));
    }
  }, [callCommand, resetPairingFlow, setConnectedPeer, setLastMessage, setStatus]);

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
        void abortOutboundConnection();
        return;
      }

      const targetName = pairingTargetRef.current?.name;
      setConnectedPeer({
        name: payload.peer_name || "已连接设备",
        peerId: payload.peer_id,
        address: targetName ? `${targetName} · 局域网直连` : "局域网直连",
        os: inferOs(payload.peer_name || "已连接设备"),
        source: "lan",
      });
      setStatus("online");
      setLastMessage(
        payload.is_reconnect
          ? `已恢复与 ${payload.peer_name} 的连接。`
          : `已与 ${payload.peer_name} 建立连接，现在可以开始同步剪贴板了 — ${formatTime()}`,
      );
      resetPairingFlow(true);
    },
    [abortOutboundConnection, resetPairingFlow, setConnectedPeer, setLastMessage, setStatus],
  );

  const handleConnectionFailed = useCallback(
    (payload: ConnectionFailedPayload) => {
      if (outboundCancelledRef.current) {
        return;
      }

      const inboundWaitStage =
        pairingStageRef.current === "incoming_request" ||
        pairingStageRef.current === "incoming_accepting";

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
      const message = isPeerOffline(payload)
        ? peerOfflineMessage(payload.peer_name)
        : peerOfflineMessage(payload.peer_name);
      const offlinePeerId = payload.peer_id?.trim() || connectedPeer?.peerId?.trim();
      if (offlinePeerId) {
        setLanDevices((previous) => previous.filter((device) => device.peer_id !== offlinePeerId));
      }
      try {
        await callCommand("disconnect");
      } catch {
      }
      setConnectedPeer(null);
      setStatus(resolveAppStatus(Math.max(0, connectedCount - 1), false));
      showTerminalNotice(message);
      resetPairingFlow(true);
    },
    [callCommand, connectedCount, connectedPeer, resetPairingFlow, setConnectedPeer, setLanDevices, setStatus, showTerminalNotice],
  );

  const beginOutboundWaitingUi = useCallback(
    (device: Device) => {
      if (outboundCancelledRef.current || isOutboundLockedStage(pairingStageRef.current)) {
        return;
      }

      outboundCancelledRef.current = false;
      setShowPairing(true);
      setPairingTarget(device);
      setPairingStage("requesting_device");
      setPairingError(null);
      setPairingRotationHint(null);
      const helperMessage = `正在等待 ${device.name} 回应…`;
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

  const handleOutboundConnectionPending = useCallback(
    (payload: OutboundConnectionPendingPayload) => {
      beginOutboundWaitingUi(deviceFromOutboundPeer(payload));
    },
    [beginOutboundWaitingUi],
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
      setPairingHelperText("请输入对方设备上显示的 6 位配对码。");
      setPairingInput("");
      setStatus("connecting");
      setLastMessage("请输入对方设备上显示的 6 位配对码。");

      try {
        const code = await callCommand<string>("get_pairing_code");
        setPairingCode(code);
      } catch {
      }
    },
    [
      callCommand,
      setLastMessage,
      setPairingCode,
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
    handleOutboundConnectionPending,
    handlePairingCodeNeeded,
    switchConnectionTarget,
    confirmSwitchConnection,
    cancelSwitchConnection,
    pairingStageRef,
    connectionLocked,
  };
}
