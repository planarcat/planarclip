import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useOverlayLifecycle } from "../useOverlayLifecycle";

// useOverlayLifecycle 负责『开-显示-关-淡出-卸载』的时序：
//   open=true  → mounted=true, exiting=false
//   open=false → mounted 保持 true 并进入 exiting=true 一段时间，超时后 mounted=false
// 常用于 PairingModal / IncomingConnectionPrompt 淡出动画。
describe("useOverlayLifecycle", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("初始 open=false 时未挂载", () => {
    const { result } = renderHook(() => useOverlayLifecycle(false));
    expect(result.current.mounted).toBe(false);
    expect(result.current.exiting).toBe(false);
  });

  it("初始 open=true 时立即挂载", () => {
    const { result } = renderHook(() => useOverlayLifecycle(true));
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);
  });

  it("关闭时先进入 exiting，超时后卸载", () => {
    const { result, rerender } = renderHook(
      ({ open }: { open: boolean }) => useOverlayLifecycle(open, 200),
      { initialProps: { open: true } },
    );

    rerender({ open: false });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(true);

    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(result.current.mounted).toBe(false);
    expect(result.current.exiting).toBe(false);
  });

  it("退出动画期间再次 open=true 立即取消退出", () => {
    const { result, rerender } = renderHook(
      ({ open }: { open: boolean }) => useOverlayLifecycle(open, 200),
      { initialProps: { open: true } },
    );

    rerender({ open: false });
    expect(result.current.exiting).toBe(true);

    // 中途再次打开：立即回到 open 状态，不等定时器
    rerender({ open: true });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);

    // 之前挂起的定时器不应再让 mounted 变 false
    act(() => {
      vi.advanceTimersByTime(500);
    });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);
  });
});