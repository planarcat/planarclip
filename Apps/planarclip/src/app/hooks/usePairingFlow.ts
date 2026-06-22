import { useCallback, useEffect, useRef } from "react";
import type {
  AppConnectionStatus,
  CommandExecutor,
  ConnectedPeer,
  ConnectionEndedPayload,
  ConnectionEstablishedPayload,
  ConnectionFailedPayload,
  ConnectionRequestPayload,
  Device,
  PairingStage,
} from "../types";
import { inferOs } from "../utils/device";
import { normalizeUserMessage } from "../utils/message";
import { formatTime } from "../utils/time";

type UsePairingFlowOptions = {
  callCommand: CommandExecutor;
  status: AppConnectionStatus;
  pairingInput: string;
  pairingStage: PairingStage;
  pairingTargetName: string | null;
  incomingRequest: ConnectionRequestPayload | null;
  setStatus: (status: AppConnectionStatus) => void;
  setLastMessage: (message: string) => void;
  setConnectedPeer: (peer: ConnectedPeer | null) => void;
  setShowPairing: (show: boolean) => void;
  setPairingInput: (value: string) => void;
  setPairingStage: (stage: PairingStage) => void;
  setPairingTargetName: (targetName: string | null) => void;
  setPairingHelperText: (message: string) => void;
  setPairingError: (message: string | null) => void;
  setIncomingRequest: (payload: ConnectionRequestPayload | null) => void;
};

/**
 * 管理配对弹层状态、手动配对、局域网连接与事件驱动的连接结果回写。
 * 输入：配对相关状态、连接状态 setter 与桌面命令执行器。
 * 输出：配对交互 handler，以及供连接桥接层调用的事件处理函数。
 */
export function usePairingFlow({
  callCommand,
  status,
  pairingInput,
  pairingStage,
  pairingTargetName,
  incomingRequest,
  setStatus,
  setLastMessage,
  setConnectedPeer,
  setShowPairing,
  setPairingInput,
  setPairingStage,
  setPairingTargetName,
  setPairingHelperText,
  setPairingError,
  setIncomingRequest,
}: UsePairingFlowOptions) {
  const pairingStageRef = useRef(pairingStage);
  const pairingTargetRef = useRef<string | null>(pairingTargetName);

  useEffect(() => {
    pairingStageRef.current = pairingStage;
  }, [pairingStage]);

  useEffect(() => {
    pairingTargetRef.current = pairingTargetName;
  }, [pairingTargetName]);

  const resetPairingFlow = useCallback(
    (closeModal = false) => {
      setPairingInput("");
      setPairingStage("idle");
      setPairingTargetName(null);
      setPairingHelperText("通过配对码或局域网设备建立连接。");
      setPairingError(null);
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
      setPairingStage,
      setPairingTargetName,
      setShowPairing,
    ],
  );

  const openPairingModal = useCallback(() => {
    setShowPairing(true);
  }, [setShowPairing]);

  const closePairingModal = useCallback(async () => {
    if (incomingRequest) {
      try {
        await callCommand("reject_connection");
        setLastMessage("已拒绝这次连接请求。");
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "拒绝连接时出了点问题，请稍后再试。"));
      }
      resetPairingFlow(true);
      setStatus("offline");
      return;
    }

    if (pairingStage === "awaiting_code" || pairingStage === "requesting_device" || pairingStage === "submitting_code") {
      try {
        await callCommand("disconnect");
      } catch {
      }
      setLastMessage("已取消本次连接，你可以重新选择附近设备。");
      setStatus("offline");
      resetPairingFlow(true);
      return;
    }

    resetPairingFlow(true);
  }, [callCommand, incomingRequest, pairingStage, resetPairingFlow, setLastMessage, setStatus]);

  const handleManualPair = useCallback(async () => {
    if (pairingInput.length !== 6) {
      setPairingError("请输入 6 位数字配对码。");
      return;
    }

    setPairingStage("manual_pairing");
    setPairingError(null);
    setPairingHelperText(`正在根据配对码 ${pairingInput} 建立连接…`);
    setStatus("connecting");
    setLastMessage(`正在根据配对码 ${pairingInput} 建立连接…`);

    try {
      await callCommand<string>("pair", { code: pairingInput });
      setConnectedPeer({
        name: "已配对设备",
        address: `配对码 ${pairingInput}`,
        os: "windows",
        source: "pair",
      });
      setStatus("online");
      setLastMessage(`已完成配对，连接已建立 — ${formatTime()}`);
      resetPairingFlow(true);
    } catch (error) {
      const message = normalizeUserMessage(error, "这次配对没有成功，请稍后再试。");
      setPairingStage("error");
      setPairingError(message);
      setPairingHelperText(message);
      setStatus("offline");
      setLastMessage(message);
    }
  }, [
    callCommand,
    pairingInput,
    resetPairingFlow,
    setConnectedPeer,
    setLastMessage,
    setPairingError,
    setPairingHelperText,
    setPairingStage,
    setStatus,
  ]);

  const handleConnectLan = useCallback(
    async (device: Device) => {
      if (!device.host || !device.port) {
        setPairingError("当前设备缺少连接地址，请等待下一轮发现结果。");
        return;
      }

      if (status === "online") {
        setPairingError("当前已经建立连接，如需切换设备，请先断开当前连接。");
        return;
      }

      setShowPairing(true);
      setPairingTargetName(device.name);
      setPairingStage("requesting_device");
      setPairingError(null);
      setPairingHelperText(`正在请求连接 ${device.name}，请稍候…`);
      setStatus("connecting");
      setLastMessage(`正在请求连接 ${device.name}，请稍候…`);

      try {
        const result = await callCommand<string>("connect_lan", { ip: device.host, port: device.port });
        if (result === "awaiting_code") {
          const message = `请查看 ${device.name} 屏幕上的 6 位配对码，并在这里输入。`;
          setPairingStage("awaiting_code");
          setPairingHelperText(message);
          setLastMessage(message);
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
        const message = normalizeUserMessage(error, `暂时无法连接 ${device.name}，请稍后重试。`, device.name);
        setPairingStage("error");
        setPairingError(message);
        setPairingHelperText(message);
        setStatus("offline");
        setLastMessage(message);
      }
    },
    [
      callCommand,
      resetPairingFlow,
      setConnectedPeer,
      setLastMessage,
      setPairingError,
      setPairingHelperText,
      setPairingStage,
      setPairingTargetName,
      setShowPairing,
      setStatus,
      status,
    ],
  );

  const handleSubmitPairingCode = useCallback(async () => {
    if (pairingInput.length !== 6) {
      setPairingError("请输入 6 位数字配对码。");
      return;
    }

    setPairingStage("submitting_code");
    setPairingError(null);
    setStatus("connecting");

    try {
      await callCommand<string>("submit_pairing_code", { code: pairingInput });
    } catch (error) {
      const message = normalizeUserMessage(
        error,
        "这次连接没有成功，请重新发起连接。",
        pairingTargetRef.current ?? undefined,
      );
      setPairingStage("error");
      setPairingError(message);
      setPairingHelperText(message);
      setStatus("offline");
      setLastMessage(message);
    }
  }, [callCommand, pairingInput, setLastMessage, setPairingError, setPairingHelperText, setPairingStage, setStatus]);

  const handleRejectIncoming = useCallback(async () => {
    try {
      await callCommand("reject_connection");
      setLastMessage("已拒绝这次连接请求。");
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "拒绝连接时出了点问题，请稍后再试。"));
    }
    resetPairingFlow(true);
    setStatus("offline");
  }, [callCommand, resetPairingFlow, setLastMessage, setStatus]);

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
      setPairingTargetName(payload.device_name);
      setPairingStage("incoming_request");
      setPairingError(null);
      setPairingHelperText(`请在 ${payload.device_name} 上输入下方配对码，或直接拒绝这次连接。`);
      setShowPairing(true);
      setStatus("connecting");
      setLastMessage(`${payload.device_name} 正在请求连接，请核对配对码后决定是否继续。`);
    },
    [
      setIncomingRequest,
      setLastMessage,
      setPairingError,
      setPairingHelperText,
      setPairingStage,
      setPairingTargetName,
      setShowPairing,
      setStatus,
    ],
  );

  const handleConnectionEstablished = useCallback(
    (payload: ConnectionEstablishedPayload) => {
      setConnectedPeer({
        name: payload.peer_name || "已连接设备",
        peerId: payload.peer_id,
        address: pairingTargetRef.current ? `${pairingTargetRef.current} · 局域网直连` : "局域网直连",
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
    [resetPairingFlow, setConnectedPeer, setLastMessage, setStatus],
  );

  const handleConnectionFailed = useCallback(
    (payload: ConnectionFailedPayload) => {
      const message = normalizeUserMessage(
        payload,
        "这次连接没有成功，请重新发起连接。",
        pairingTargetRef.current ?? undefined,
      );
      setPairingStage("error");
      setPairingError(message);
      setPairingHelperText(message);
      setStatus("offline");
      setLastMessage(message);
    },
    [setLastMessage, setPairingError, setPairingHelperText, setPairingStage, setStatus],
  );

  const handleConnectionEnded = useCallback(
    (payload: ConnectionEndedPayload) => {
      const message = normalizeUserMessage(payload, "连接已断开，请重新连接。", payload.peer_name);
      setConnectedPeer(null);
      setStatus("offline");
      setLastMessage(message);
      resetPairingFlow(true);
    },
    [resetPairingFlow, setConnectedPeer, setLastMessage, setStatus],
  );

  return {
    openPairingModal,
    closePairingModal,
    handleManualPair,
    handleConnectLan,
    handleSubmitPairingCode,
    handleRejectIncoming,
    handleDisconnect,
    handleConnectionRequest,
    handleConnectionEstablished,
    handleConnectionFailed,
    handleConnectionEnded,
    pairingStageRef,
  };
}
