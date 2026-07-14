import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useRelativeTicker } from "../useRelativeTicker";

describe("useRelativeTicker", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("每 5 秒触发一次重渲染", () => {
    const { rerender } = renderHook(() => useRelativeTicker());

    const initialRenderCount = vi.getTimerCount();

    act(() => {
      vi.advanceTimersByTime(5_000);
    });

    // 应触发一次 setTick 使组件重渲染
    expect(vi.getTimerCount()).toBe(initialRenderCount);
  });

  it("卸载时清除定时器", () => {
    const { unmount } = renderHook(() => useRelativeTicker());

    expect(vi.getTimerCount()).toBe(1);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});
