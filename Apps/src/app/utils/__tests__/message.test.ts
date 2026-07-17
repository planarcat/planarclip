import { describe, expect, it } from "vitest";

import {
  MSG_INVALID_PAIRING_CODE,
  MSG_PAIRING_CODE_REFRESHED,
  MSG_PEER_CANCELLED,
  MSG_PEER_REJECTED,
  MSG_PEER_RESPONSE_TIMEOUT,
  MSG_SELF_CANCELLED_OUTBOUND,
  MSG_SELF_INCOMING_TIMEOUT,
  connectionEndedMessage,
  connectionUnavailableMessage,
  isConnectionRejected,
  isConnectionTimeout,
  isInvalidPairingCode,
  isPeerCancelled,
  isPeerOffline,
  normalizeUserMessage,
  peerDisconnectedMessage,
  peerOfflineMessage,
  rawMessage,
} from "../message";

// 这些函数负责把 Rust 侧的错误 / peer:* 回应码 映射成用户可见文案，
// 属于跨端协议契约的前端一侧，回归价值最高。

describe("rawMessage", () => {
  it("从 { message } 对象提取", () => {
    expect(rawMessage({ message: " hello " })).toBe("hello");
  });

  it("其他情况 stringify", () => {
    expect(rawMessage("plain")).toBe("plain");
    expect(rawMessage(null)).toBe("");
    expect(rawMessage(undefined)).toBe("");
    expect(rawMessage(42)).toBe("42");
  });
});

describe("isConnectionRejected", () => {
  it("kind 分支：rejected / timeout / cancelled / peer_cancelled 全命中", () => {
    for (const kind of ["rejected", "timeout", "cancelled", "peer_cancelled"]) {
      expect(isConnectionRejected({ kind })).toBe(true);
    }
  });

  it("文案分支：常见中文关键字命中", () => {
    expect(isConnectionRejected("对方已取消这次连接")).toBe(true);
    expect(isConnectionRejected("连接请求已超时")).toBe(true);
    expect(isConnectionRejected("你已取消这次连接")).toBe(true);
  });

  it("无关文案返回 false", () => {
    expect(isConnectionRejected("协议错误")).toBe(false);
  });
});

describe("isConnectionTimeout / isPeerCancelled / isPeerOffline / isInvalidPairingCode", () => {
  it("各自识别对应 kind", () => {
    expect(isConnectionTimeout({ kind: "timeout" })).toBe(true);
    expect(isPeerCancelled({ kind: "peer_cancelled" })).toBe(true);
    expect(isPeerOffline({ kind: "peer_offline" })).toBe(true);
    expect(isInvalidPairingCode({ kind: "invalid_code" })).toBe(true);
  });

  it("识别中文文案分支", () => {
    expect(isConnectionTimeout("这次连接已超时")).toBe(true);
    expect(isPeerCancelled("对方已取消这次连接")).toBe(true);
    expect(isPeerOffline("对方已下线")).toBe(true);
    expect(isInvalidPairingCode("配对码无效")).toBe(true);
  });
});

describe("peerOfflineMessage / peerDisconnectedMessage / connectionUnavailableMessage", () => {
  it("带名字与不带名字两种形态", () => {
    expect(peerOfflineMessage("A")).toBe("A 已下线。");
    expect(peerOfflineMessage()).toBe("对方设备已下线。");
    expect(peerDisconnectedMessage("A")).toBe("与 A 的连接已断开。");
    expect(peerDisconnectedMessage()).toBe("与对方设备的连接已断开。");
    expect(connectionUnavailableMessage("A")).toBe("连接失败，请确认本机与 A 的网络状态。");
    expect(connectionUnavailableMessage()).toBe("连接失败，请确认本机与对方的网络状态。");
  });
});

describe("connectionEndedMessage", () => {
  it("peer_offline kind 优先使用离线文案", () => {
    expect(connectionEndedMessage({ kind: "peer_offline", peer_name: "A" })).toBe("A 已下线。");
  });

  it("有 message 时走归一化", () => {
    // 底层 raw 「对方已取消」→ MSG_PEER_CANCELLED
    expect(
      connectionEndedMessage({ message: "对方已取消这次连接", peer_name: "A" }),
    ).toBe(MSG_PEER_CANCELLED);
  });

  it("无 message 时兜底为断开连接文案", () => {
    expect(connectionEndedMessage({ peer_name: "A" })).toBe("与 A 的连接已断开。");
  });
});

describe("normalizeUserMessage — peer:* 回应码分发（协议契约）", () => {
  const fallback = "FALLBACK";

  it("对方取消 → MSG_PEER_CANCELLED", () => {
    expect(normalizeUserMessage("对方已取消这次连接", fallback)).toBe(MSG_PEER_CANCELLED);
  });

  it("对方拒绝 → MSG_PEER_REJECTED", () => {
    expect(normalizeUserMessage("对方拒绝了这次连接", fallback)).toBe(MSG_PEER_REJECTED);
    expect(normalizeUserMessage("对方已拒绝这次连接", fallback)).toBe(MSG_PEER_REJECTED);
  });

  it("请求超时 → MSG_PEER_RESPONSE_TIMEOUT", () => {
    expect(normalizeUserMessage("连接请求已超时", fallback)).toBe(MSG_PEER_RESPONSE_TIMEOUT);
    expect(normalizeUserMessage("这次连接已超时", fallback)).toBe(MSG_PEER_RESPONSE_TIMEOUT);
  });

  it("配对码过期 → MSG_PAIRING_CODE_REFRESHED", () => {
    expect(normalizeUserMessage("配对码已过期", fallback)).toBe(MSG_PAIRING_CODE_REFRESHED);
  });

  it("配对码无效 → MSG_INVALID_PAIRING_CODE", () => {
    expect(normalizeUserMessage("配对码无效", fallback)).toBe(MSG_INVALID_PAIRING_CODE);
    expect(normalizeUserMessage("配对码不正确", fallback)).toBe(MSG_INVALID_PAIRING_CODE);
  });

  it("本机自取消 → MSG_SELF_CANCELLED_OUTBOUND", () => {
    expect(normalizeUserMessage("你已取消这次连接", fallback)).toBe(MSG_SELF_CANCELLED_OUTBOUND);
    expect(normalizeUserMessage("已取消这次连接", fallback)).toBe(MSG_SELF_CANCELLED_OUTBOUND);
  });

  it("入站未及时回应 → MSG_SELF_INCOMING_TIMEOUT", () => {
    expect(normalizeUserMessage("未及时回应", fallback)).toBe(MSG_SELF_INCOMING_TIMEOUT);
  });

  it("网络不可达类 → connectionUnavailableMessage", () => {
    expect(normalizeUserMessage("os error 10061", fallback, "A")).toBe(
      "连接失败，请确认本机与 A 的网络状态。",
    );
    expect(normalizeUserMessage("actively refused", fallback)).toBe(
      "连接失败，请确认本机与对方的网络状态。",
    );
  });

  it("已下线 → peerOfflineMessage", () => {
    expect(normalizeUserMessage("对方已下线", fallback, "A")).toBe("A 已下线。");
  });

  it("剥离 `连接失败：` / `配对失败：` 前缀后递归", () => {
    expect(normalizeUserMessage("连接失败：对方已取消这次连接", fallback)).toBe(MSG_PEER_CANCELLED);
  });

  it("完全不认识的文案回退 fallback", () => {
    expect(normalizeUserMessage("未知错误 X-42", fallback)).toBe(fallback);
  });
});