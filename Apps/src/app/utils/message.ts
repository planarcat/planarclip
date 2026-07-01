export function rawMessage(error: unknown) {
  if (error && typeof error === "object" && "message" in error) {
    const { message } = error as { message?: unknown };
    if (typeof message === "string" && message.trim()) {
      return message.trim();
    }
  }

  return String(error ?? "").trim();
}

export function isConnectionRejected(error: unknown) {
  if (error && typeof error === "object" && "kind" in error) {
    const { kind } = error as { kind?: unknown };
    if (kind === "rejected" || kind === "timeout" || kind === "cancelled" || kind === "peer_cancelled") {
      return true;
    }
  }

  const raw = rawMessage(error);
  return (
    raw.includes("对方已取消") ||
    raw.includes("对方已拒绝") ||
    raw.includes("对方拒绝了") ||
    raw.includes("连接请求已超时") ||
    raw.includes("这次连接已超时") ||
    raw.includes("你已取消") ||
    raw.includes("已取消这次连接")
  );
}

export function isConnectionTimeout(error: unknown) {
  if (error && typeof error === "object" && "kind" in error) {
    return (error as { kind?: unknown }).kind === "timeout";
  }
  const raw = rawMessage(error);
  return raw.includes("连接请求已超时") || raw.includes("这次连接已超时");
}

export function isInvalidPairingCode(error: unknown) {
  if (error && typeof error === "object" && "kind" in error) {
    return (error as { kind?: unknown }).kind === "invalid_code";
  }
  const raw = rawMessage(error);
  return raw.includes("配对码无效") || raw.includes("配对码不正确");
}

export function isPeerCancelled(error: unknown) {
  if (error && typeof error === "object" && "kind" in error) {
    return (error as { kind?: unknown }).kind === "peer_cancelled";
  }
  const raw = rawMessage(error);
  return raw.includes("对方已取消这次连接");
}

export function isPeerOffline(error: unknown) {
  if (error && typeof error === "object" && "kind" in error) {
    const kind = (error as { kind?: unknown }).kind;
    return kind === "peer_offline";
  }
  const raw = rawMessage(error);
  return raw.includes("已下线");
}

/** 对方主动取消连接 */
export const MSG_PEER_CANCELLED = "对方已取消这次连接。";

/** 主动发起连接时，对方明确拒绝 */
export const MSG_PEER_REJECTED = "对方拒绝了这次连接。";

/** 等待对方回应超时 */
export const MSG_PEER_RESPONSE_TIMEOUT = "对方未及时回应，这次连接已超时。";

/** 本机关闭窗口取消出站连接 */
export const MSG_SELF_CANCELLED_OUTBOUND = "你已取消这次连接。";

/** 本机主动取消入站配对或连接流程 */
export const MSG_SELF_CANCELLED_INBOUND = "你已取消这次连接。";

/** 本机未及时回应入站连接确认（倒计时超时） */
export const MSG_SELF_INCOMING_TIMEOUT = "未及时回应，这次连接已超时。";

/** 配对码错误 */
export const MSG_INVALID_PAIRING_CODE = "配对码错误，请重新输入。";

/** 配对码输入阶段超时后轮换 */
export const MSG_PAIRING_CODE_REFRESHED = "配对码已更新，请重新输入。";

/** 发起方：等待输入对方屏幕上的配对码 */
export const MSG_ENTER_PEER_PAIRING_CODE = "请输入对方设备上显示的 6 位配对码。";

/** 接收方：等待对方输入本机配对码 */
export const MSG_WAIT_FOR_PEER_PAIRING_CODE =
  "请让对方在他的设备上输入你屏幕上的配对码。";

/** 连接数已达上限 */
export const MSG_CONNECTION_LIMIT =
  "已超出连接上限，请先断开其中一个设备后再连接。";

/** 网络类连接失败 */
export function connectionUnavailableMessage(targetName?: string) {
  if (targetName) {
    return `连接失败，请确认本机与 ${targetName} 的网络状态。`;
  }
  return "连接失败，请确认本机与对方的网络状态。";
}

/** 连接已断开（对端仍可能在线） */
export const MSG_PEER_DISCONNECTED = "与对方设备的连接已断开。";

/** 对方下线 */
export function peerOfflineMessage(peerName?: string) {
  if (peerName?.trim()) {
    return `${peerName.trim()} 已下线。`;
  }
  return "对方设备已下线。";
}

export function peerDisconnectedMessage(peerName?: string) {
  if (peerName?.trim()) {
    return `与 ${peerName.trim()} 的连接已断开。`;
  }
  return MSG_PEER_DISCONNECTED;
}

export function connectionEndedMessage(payload: {
  kind?: string;
  message?: string;
  peer_name?: string;
}) {
  if (payload.kind === "peer_offline") {
    return peerOfflineMessage(payload.peer_name);
  }

  if (payload.message?.trim()) {
    return normalizeUserMessage(
      payload,
      peerDisconnectedMessage(payload.peer_name),
      payload.peer_name,
    );
  }

  return peerDisconnectedMessage(payload.peer_name);
}

export function normalizeUserMessage(error: unknown, fallback: string, targetName?: string) {
  if (error && typeof error === "object" && "kind" in error) {
    const { kind } = error as { kind?: unknown };
    if (kind === "timeout") {
      return MSG_PEER_RESPONSE_TIMEOUT;
    }
    if (kind === "peer_cancelled" || kind === "cancelled") {
      return MSG_PEER_CANCELLED;
    }
    if (kind === "rejected") {
      return MSG_PEER_REJECTED;
    }
  }

  const raw = rawMessage(error);

  if (!raw) {
    return fallback;
  }

  if (raw.includes("已超出连接上限")) {
    return MSG_CONNECTION_LIMIT;
  }

  if (raw.includes("配对码必须为 6 位数字")) {
    return "请输入 6 位数字配对码。";
  }

  if (raw.includes("当前没有待处理的连接")) {
    return "这次连接已经结束，请重新选择设备后再试。";
  }

  if (raw.includes("密钥对尚未初始化")) {
    return "设备还在准备连接信息，请稍后再试。";
  }

  if (raw.includes("对方已取消")) {
    return MSG_PEER_CANCELLED;
  }

  if (raw.includes("对方已拒绝") || raw.includes("对方拒绝了")) {
    return MSG_PEER_REJECTED;
  }

  if (raw.includes("连接请求已超时") || raw.includes("这次连接已超时")) {
    return MSG_PEER_RESPONSE_TIMEOUT;
  }

  if (raw.includes("配对码已过期")) {
    return MSG_PAIRING_CODE_REFRESHED;
  }

  if (raw.includes("配对码无效") || raw.includes("配对码不正确")) {
    return MSG_INVALID_PAIRING_CODE;
  }

  if (raw.includes("未及时回应") || raw.includes("已自动拒绝")) {
    return MSG_SELF_INCOMING_TIMEOUT;
  }

  if (raw.includes("你已取消") || raw.includes("已取消这次连接")) {
    return MSG_SELF_CANCELLED_OUTBOUND;
  }

  if (raw.includes("用户已取消") || raw.includes("已取消")) {
    return MSG_SELF_CANCELLED_OUTBOUND;
  }

  if (raw.includes("已下线")) {
    return peerOfflineMessage(targetName);
  }

  if (raw.includes("连接已断开") || raw.includes("连接已中断") || raw.includes("已断开连接")) {
    return peerDisconnectedMessage(targetName);
  }

  if (
    raw.includes("I/O 错误") ||
    raw.includes("I/O error") ||
    raw.includes("os error 10061") ||
    raw.includes("无法连接") ||
    raw.includes("actively refused") ||
    raw.includes("暂时无法连接对方设备")
  ) {
    return connectionUnavailableMessage(targetName);
  }

  if (raw.includes("协议错误") || raw.includes("帧错误") || raw.includes("连接过程中出了点问题")) {
    return connectionUnavailableMessage(targetName);
  }

  if (raw.startsWith("连接失败：") || raw.startsWith("配对失败：")) {
    return normalizeUserMessage(raw.replace(/^[^：]+：/, "").trim(), fallback, targetName);
  }

  if (raw.includes("浏览器预览模式") || raw.includes("桌面端")) {
    return "当前是浏览器预览模式，请在桌面应用中体验连接能力。";
  }

  return fallback;
}
