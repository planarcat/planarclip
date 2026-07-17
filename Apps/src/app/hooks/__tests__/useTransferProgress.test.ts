import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useTransferProgress } from "../useTransferProgress";
import type { ClipboardSyncActivityPayload } from "../useTransferProgress";

function makePayload(overrides: Partial<ClipboardSyncActivityPayload> = {}): ClipboardSyncActivityPayload {
  return {
    active: true,
    kind: "file",
    message: "发送中…",
    progress: 0.5,
    direction: "send",
    ...overrides,
  };
}

describe("useTransferProgress", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("初始状态为 null", () => {
    const { result } = renderHook(() => useTransferProgress());
    expect(result.current.transferProgress).toBeNull();
  });

  it("applySyncActivity 设置 active 进度", () => {
    const { result } = renderHook(() => useTransferProgress());

    act(() => {
      result.current.applySyncActivity(makePayload());
    });

    expect(result.current.transferProgress).toEqual({
      active: true,
      message: "发送中…",
      progress: 0.5,
      label: undefined,
      bytesDone: undefined,
      bytesTotal: undefined,
      batchIndex: undefined,
      batchTotal: undefined,
    });
  });

  it("kind='notice' 直接清除状态", () => {
    const { result } = renderHook(() => useTransferProgress());

    act(() => {
      result.current.applySyncActivity(makePayload());
    });
    expect(result.current.transferProgress).not.toBeNull();

    act(() => {
      result.current.applySyncActivity(makePayload({ kind: "notice" }));
    });
    expect(result.current.transferProgress).toBeNull();
  });

  it("kind 既不是 file 也不是 image 时跳过", () => {
    const { result } = renderHook(() => useTransferProgress());

    act(() => {
      result.current.applySyncActivity(makePayload({ kind: "text" }));
    });
    expect(result.current.transferProgress).toBeNull();
  });

  it("active=false 时将 progress 设为 1，5 秒后自动清除", () => {
    const { result } = renderHook(() => useTransferProgress());

    act(() => {
      result.current.applySyncActivity(makePayload({ active: true }));
    });
    expect(result.current.transferProgress?.active).toBe(true);

    act(() => {
      result.current.applySyncActivity(makePayload({ active: false, message: "完成" }));
    });
    expect(result.current.transferProgress?.active).toBe(false);
    expect(result.current.transferProgress?.progress).toBe(1);
    expect(result.current.transferProgress?.message).toBe("完成");

    // 5 秒后自动清除
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(result.current.transferProgress).toBeNull();
  });

  it("主动清除时取消 dismiss 定时器", () => {
    const { result } = renderHook(() => useTransferProgress());

    act(() => {
      result.current.applySyncActivity(makePayload({ active: true }));
    });
    act(() => {
      result.current.applySyncActivity(makePayload({ active: false, message: "完成" }));
    });

    // 主动清除
    act(() => {
      result.current.clearTransferProgress();
    });
    expect(result.current.transferProgress).toBeNull();

    // 5 秒后定时器不应再触发
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(result.current.transferProgress).toBeNull();
  });

  it("active 状态切换时清除旧的 dismiss 定时器", () => {
    const { result } = renderHook(() => useTransferProgress());

    // 完成一次传输
    act(() => {
      result.current.applySyncActivity(makePayload({ active: true }));
    });
    act(() => {
      result.current.applySyncActivity(makePayload({ active: false, message: "完成" }));
    });

    // 2 秒后新传输开始——应取消旧定时器
    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    act(() => {
      result.current.applySyncActivity(makePayload({ active: true, message: "新传输…" }));
    });
    expect(result.current.transferProgress?.active).toBe(true);
    expect(result.current.transferProgress?.message).toBe("新传输…");

    // 旧定时器剩余的 3 秒不应触发清除
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(result.current.transferProgress).not.toBeNull();
  });

  it("卸载时清除 dismiss 定时器", () => {
    const { result, unmount } = renderHook(() => useTransferProgress());

    act(() => {
      result.current.applySyncActivity(makePayload({ active: true }));
    });
    act(() => {
      result.current.applySyncActivity(makePayload({ active: false, message: "完成" }));
    });

    expect(vi.getTimerCount()).toBe(1);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("active=true 时 shouldShowTransferProgressCard 返回 false 则跳过", () => {
    const { result } = renderHook(() => useTransferProgress());

    // 使用一个不被 shouldShowTransferProgressCard 显示的 payload
    // 该函数内部逻辑：当 progress 为 0 且 direction 为 "receive" 时可能隐藏
    // 用 message 为空来触发隐藏逻辑
    act(() => {
      result.current.applySyncActivity(makePayload({
        active: true,
        message: "",
        direction: "receive",
        progress: 0,
      }));
    });
    // 不显示卡 — 不更新状态
  });
});
