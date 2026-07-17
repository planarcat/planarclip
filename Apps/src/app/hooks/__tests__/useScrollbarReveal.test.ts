import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useScrollbarReveal } from "../useScrollbarReveal";

function createMockElement() {
  const classList = {
    add: vi.fn(),
    remove: vi.fn(),
  };
  const listeners = new Map<string, () => void>();
  const el = {
    classList,
    addEventListener: vi.fn((event: string, handler: () => void) => {
      listeners.set(event, handler);
    }),
    removeEventListener: vi.fn((event: string) => {
      listeners.delete(event);
    }),
  };
  return { el: el as unknown as HTMLElement, listeners };
}

describe("useScrollbarReveal", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("root 为 null 时不注册事件", () => {
    renderHook(() => useScrollbarReveal(null));
    // 无副作用，不报错
  });

  it("root 不为 null 时注册 pointerenter/pointerleave/scroll 事件", () => {
    const { el } = createMockElement();
    renderHook(() => useScrollbarReveal(el));

    expect(el.addEventListener).toHaveBeenCalledWith("pointerenter", expect.any(Function));
    expect(el.addEventListener).toHaveBeenCalledWith("pointerleave", expect.any(Function));
    expect(el.addEventListener).toHaveBeenCalledWith("scroll", expect.any(Function), { passive: true });
  });

  it("pointerenter 添加 engaged class，移除 leaving class", () => {
    const { el, listeners } = createMockElement();
    renderHook(() => useScrollbarReveal(el));

    const onPointerEnter = listeners.get("pointerenter")!;
    act(() => {
      onPointerEnter();
    });

    expect(el.classList.remove).toHaveBeenCalledWith("app-scrollbar-leaving");
    expect(el.classList.add).toHaveBeenCalledWith("app-scrollbar-engaged");
  });

  it("pointerleave 移除 engaged 并添加 leaving，520ms 后移除 leaving", () => {
    const { el, listeners } = createMockElement();
    renderHook(() => useScrollbarReveal(el));

    const onPointerLeave = listeners.get("pointerleave")!;
    act(() => {
      onPointerLeave();
    });

    expect(el.classList.remove).toHaveBeenCalledWith("app-scrollbar-engaged");
    expect(el.classList.add).toHaveBeenCalledWith("app-scrollbar-leaving");

    act(() => {
      vi.advanceTimersByTime(520);
    });
    expect(el.classList.remove).toHaveBeenCalledWith("app-scrollbar-leaving");
  });

  it("scroll 触发 engaged，800ms 无操作后开始 leaving", () => {
    const { el, listeners } = createMockElement();
    renderHook(() => useScrollbarReveal(el));

    const onScroll = listeners.get("scroll")!;
    act(() => {
      onScroll();
    });

    expect(el.classList.add).toHaveBeenCalledWith("app-scrollbar-engaged");

    act(() => {
      vi.advanceTimersByTime(800);
    });
    expect(el.classList.remove).toHaveBeenCalledWith("app-scrollbar-engaged");
    expect(el.classList.add).toHaveBeenCalledWith("app-scrollbar-leaving");
  });

  it("卸载时清除事件监听和定时器", () => {
    const { el } = createMockElement();
    const { unmount } = renderHook(() => useScrollbarReveal(el));

    unmount();

    expect(el.removeEventListener).toHaveBeenCalledWith("pointerenter", expect.any(Function));
    expect(el.removeEventListener).toHaveBeenCalledWith("pointerleave", expect.any(Function));
    expect(el.removeEventListener).toHaveBeenCalledWith("scroll", expect.any(Function));
    expect(el.classList.remove).toHaveBeenCalledWith("app-scrollbar-engaged", "app-scrollbar-leaving");
    expect(vi.getTimerCount()).toBe(0);
  });
});
