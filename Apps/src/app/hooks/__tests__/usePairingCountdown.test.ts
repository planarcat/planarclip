import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  PAIRING_COUNTDOWN_SECS,
  PAIRING_URGENT_THRESHOLD_SECS,
  usePairingCountdown,
} from "../usePairingCountdown";

// 该 hook 是配对弹层 60s 倒计时 + 到期回调（触发配对码轮换）的核心。
// 语义关键：
//  - active=false 时不启动定时器且始终显示满时长
//  - 到期 (prev<=1) 时调用 onExpire 并**立即重置**为 durationSecs，形成轮换
//  - isUrgent = 剩余 ≤ 10s
describe("usePairingCountdown", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("默认时长为 PAIRING_COUNTDOWN_SECS", () => {
    const { result } = renderHook(() =>
      usePairingCountdown({ active: false, onExpire: () => {} }),
    );
    expect(result.current.remainingSeconds).toBe(PAIRING_COUNTDOWN_SECS);
    expect(result.current.progress).toBe(1);
    expect(result.current.isUrgent).toBe(false);
  });

  it("active=false 时定时器不推进", () => {
    const onExpire = vi.fn();
    const { result } = renderHook(() =>
      usePairingCountdown({ active: false, onExpire, durationSecs: 5 }),
    );
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(result.current.remainingSeconds).toBe(5);
    expect(onExpire).not.toHaveBeenCalled();
  });

  it("active=true 时每秒递减", () => {
    const { result } = renderHook(() =>
      usePairingCountdown({ active: true, onExpire: () => {}, durationSecs: 5 }),
    );
    expect(result.current.remainingSeconds).toBe(5);

    act(() => {
      vi.advanceTimersByTime(1_000);
    });
    expect(result.current.remainingSeconds).toBe(4);

    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    expect(result.current.remainingSeconds).toBe(2);
  });

  it("到期触发 onExpire 并轮换回满时长", () => {
    const onExpire = vi.fn();
    const { result } = renderHook(() =>
      usePairingCountdown({ active: true, onExpire, durationSecs: 3 }),
    );

    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(onExpire).toHaveBeenCalledTimes(1);
    // 到 0 后立即重置为 durationSecs（配对码轮换语义）
    expect(result.current.remainingSeconds).toBe(3);

    // 再一整轮：应再次触发
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(onExpire).toHaveBeenCalledTimes(2);
  });

  it("isUrgent 在剩余秒 ≤ PAIRING_URGENT_THRESHOLD_SECS 时为 true", () => {
    const duration = PAIRING_URGENT_THRESHOLD_SECS + 2;
    const { result } = renderHook(() =>
      usePairingCountdown({
        active: true,
        onExpire: () => {},
        durationSecs: duration,
      }),
    );
    expect(result.current.isUrgent).toBe(false);
    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    expect(result.current.remainingSeconds).toBe(PAIRING_URGENT_THRESHOLD_SECS);
    expect(result.current.isUrgent).toBe(true);
  });

  it("progress 为 remainingSeconds / durationSecs", () => {
    const { result } = renderHook(() =>
      usePairingCountdown({ active: true, onExpire: () => {}, durationSecs: 4 }),
    );
    act(() => {
      vi.advanceTimersByTime(1_000);
    });
    expect(result.current.progress).toBeCloseTo(0.75, 5);
  });

  it("active 由 true 切回 false 时重置到满时长", () => {
    const { result, rerender } = renderHook(
      (props: { active: boolean }) =>
        usePairingCountdown({ ...props, onExpire: () => {}, durationSecs: 5 }),
      { initialProps: { active: true } },
    );
    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    expect(result.current.remainingSeconds).toBe(3);

    rerender({ active: false });
    expect(result.current.remainingSeconds).toBe(5);
  });
});