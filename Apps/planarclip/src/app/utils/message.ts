export function rawMessage(error: unknown) {
  if (error && typeof error === "object" && "message" in error) {
    const { message } = error as { message?: unknown };
    if (typeof message === "string" && message.trim()) {
      return message.trim();
    }
  }

  return String(error ?? "").trim();
}

export function normalizeUserMessage(error: unknown, fallback: string, targetName?: string) {
  const raw = rawMessage(error);

  if (!raw) {
    return fallback;
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

  if (raw.includes("对方已拒绝连接") || raw.includes("对方已拒绝这次连接")) {
    return "对方没有继续这次连接，请重新发起连接。";
  }

  if (raw.includes("配对码已过期") || raw.includes("这次连接已超时")) {
    return "这次配对已超时，请重新发起连接并输入新的配对码。";
  }

  if (raw.includes("配对码无效") || raw.includes("配对码不正确")) {
    return "配对码不正确，或这次连接已经失效，请重新核对后再试。";
  }

  if (raw.includes("已取消") || raw.includes("用户已取消")) {
    return "这次连接已经取消，你可以重新选择设备。";
  }

  if (raw.includes("已断开连接")) {
    return raw;
  }

  if (
    raw.includes("I/O 错误") ||
    raw.includes("I/O error") ||
    raw.includes("os error 10061") ||
    raw.includes("无法连接") ||
    raw.includes("actively refused") ||
    raw.includes("暂时无法连接对方设备")
  ) {
    if (targetName) {
      return `暂时连不上 ${targetName}，请确认对方应用已打开，而且你们在同一局域网内。`;
    }
    return "暂时无法连接对方设备，请确认对方应用已打开，而且你们在同一局域网内。";
  }

  if (raw.includes("协议错误") || raw.includes("帧错误") || raw.includes("连接过程中出了点问题")) {
    return "连接过程中出了点问题，请重新发起连接。";
  }

  if (raw.startsWith("连接失败：") || raw.startsWith("配对失败：")) {
    return normalizeUserMessage(raw.replace(/^[^：]+：/, "").trim(), fallback, targetName);
  }

  if (raw.includes("浏览器预览模式") || raw.includes("桌面端")) {
    return "当前是浏览器预览模式，请在桌面应用中体验连接能力。";
  }

  return fallback;
}
