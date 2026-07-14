import { describe, expect, it, vi, afterEach, beforeEach } from "vitest";

import { relativeTime, formatTime } from "../time";

// 用 fake timers 固定 Date.now，让「N 秒前」类断言可重复。
describe("relativeTime", () => {
  const NOW = new Date("2026-07-11T12:00:00Z").getTime();

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(NOW);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("无参数时返回 `刚刚`", () => {
    expect(relativeTime()).toBe("刚刚");
  });

  it("秒级返回 `N秒前`", () => {
    expect(relativeTime(new Date(NOW - 5_000))).toBe("5秒前");
  });

  it("分钟级返回 `N分钟前`", () => {
    expect(relativeTime(new Date(NOW - 3 * 60 * 1000))).toBe("3分钟前");
  });

  it("小时级返回 `N小时前`", () => {
    expect(relativeTime(new Date(NOW - 2 * 3600 * 1000))).toBe("2小时前");
  });

  it("天级返回 `N天前`", () => {
    expect(relativeTime(new Date(NOW - 3 * 86400 * 1000))).toBe("3天前");
  });

  it("未来时间被夹到 0，返回 `0秒前`", () => {
    // 保护「时钟回拨」场景，避免出现负数字符串。
    expect(relativeTime(new Date(NOW + 10_000))).toBe("0秒前");
  });
});

describe("formatTime", () => {
  it("返回本地化时间字符串", () => {
    // 只做形态断言，避免对 locale 具体格式过度绑定。
    expect(typeof formatTime()).toBe("string");
    expect(formatTime().length).toBeGreaterThan(0);
  });
});