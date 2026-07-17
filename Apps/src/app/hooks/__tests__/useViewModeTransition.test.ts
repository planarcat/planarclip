import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useViewModeTransition } from "../useViewModeTransition";
import type { ViewMode } from "../../types";

describe("useViewModeTransition", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("初始 displayMode 为 targetMode，无动画 class", () => {
    const { result } = renderHook(() => useViewModeTransition("list"));
    expect(result.current.displayMode).toBe("list");
    expect(result.current.contentClass).toBe("");
  });

  it("切换到相同 mode 不触发动画", () => {
    const { result, rerender } = renderHook(
      ({ mode }: { mode: ViewMode }) => useViewModeTransition(mode),
      { initialProps: { mode: "list" as ViewMode } },
    );

    rerender({ mode: "list" });
    expect(result.current.displayMode).toBe("list");
    expect(result.current.contentClass).toBe("");
  });

  it("切换到不同 mode：先 exiting class，160ms 后切换 displayMode", () => {
    const { result, rerender } = renderHook(
      ({ mode }: { mode: ViewMode }) => useViewModeTransition(mode),
      { initialProps: { mode: "list" as ViewMode } },
    );

    rerender({ mode: "grid" });

    expect(result.current.displayMode).toBe("list");
    expect(result.current.contentClass).toBe("clip-history-view-exit");

    act(() => {
      vi.advanceTimersByTime(160);
    });

    expect(result.current.displayMode).toBe("grid");
    expect(result.current.contentClass).toBe("clip-history-view-enter");
  });

  it("连续切换：每次先进入 exiting 再重新开始", () => {
    const { result, rerender } = renderHook(
      ({ mode }: { mode: ViewMode }) => useViewModeTransition(mode),
      { initialProps: { mode: "list" as ViewMode } },
    );

    rerender({ mode: "grid" });
    expect(result.current.contentClass).toBe("clip-history-view-exit");

    rerender({ mode: "list" });
    expect(result.current.contentClass).toBe("clip-history-view-exit");
    expect(result.current.displayMode).toBe("list");

    act(() => {
      vi.advanceTimersByTime(160);
    });
    expect(result.current.displayMode).toBe("list");
  });

  it("卸载时清除定时器", () => {
    const { rerender, unmount } = renderHook(
      ({ mode }: { mode: ViewMode }) => useViewModeTransition(mode),
      { initialProps: { mode: "list" as ViewMode } },
    );

    rerender({ mode: "grid" });
    expect(vi.getTimerCount()).toBe(1);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});
