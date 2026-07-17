import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useThemeTransition } from "../useThemeTransition";

vi.mock("../../utils/themeTokenBlend", () => ({
  captureThemeTokens: vi.fn(() => ({})),
  clearThemeTokenInlineOverrides: vi.fn(),
  resolveAppearanceTokens: vi.fn(() => ({})),
  runThemeTokenBlend: vi.fn(() => {
    const stop = vi.fn();
    return stop;
  }),
}));
vi.mock("../../utils/themeTransition", () => ({
  THEME_TRANSITION_ICON_MS: 350,
  THEME_TRANSITION_ICON_EXIT_MS: 350,
  THEME_TRANSITION_EXPAND_MS: 1000,
  isColorSchemeDark: (scheme: string) => scheme === "dark",
  prefersReducedMotion: () => false,
  THEME_TRANSITION_LIGHT_BG: "#f0f4f9",
  THEME_TRANSITION_DARK_BG: "#090c14",
  THEME_TRANSITION_CIRCLE_REVEAL_ANIMATION: "theme-transition-circle-reveal",
  THEME_TRANSITION_ICON_EXIT_ANIMATION: "theme-transition-icon-exit",
}));

describe("useThemeTransition", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.stubGlobal("performance", { now: vi.fn(() => 0) });
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("初始状态：transitionState 为 null，isTransitioning 为 false", () => {
    const { result } = renderHook(() => useThemeTransition());
    expect(result.current.transitionState).toBeNull();
    expect(result.current.isTransitioning).toBe(false);
    expect(result.current.appearanceTransitionActive).toBe(false);
  });

  it("playBackgroundTransition 当 busy 时直接返回", () => {
    const { result } = renderHook(() => useThemeTransition());
    const onComplete = vi.fn();

    act(() => {
      result.current.playBackgroundTransition("dark", { id: "cyan", label: "青色", light: { primary: "#fff", accent: "#0c4a6e", ring: "#fff" }, dark: { primary: "#000", accent: "#0e7490", ring: "#000" } }, onComplete);
    });
    act(() => {
      result.current.playBackgroundTransition("light", { id: "cyan", label: "青色", light: { primary: "#fff", accent: "#0c4a6e", ring: "#fff" }, dark: { primary: "#000", accent: "#0e7490", ring: "#000" } }, vi.fn());
    });

    expect(onComplete).not.toHaveBeenCalled();
  });

  it("playBackgroundTransition 完整流程：icon → expand → iconExit → finalize", () => {
    const { result } = renderHook(() => useThemeTransition());
    const onComplete = vi.fn();

    act(() => {
      result.current.playBackgroundTransition("dark", { id: "cyan", label: "青色", light: { primary: "#fff", accent: "#0c4a6e", ring: "#fff" }, dark: { primary: "#000", accent: "#0e7490", ring: "#000" } }, onComplete);
    });

    expect(result.current.transitionState?.kind).toBe("background");
    if (result.current.transitionState?.kind === "background") {
      expect(result.current.transitionState.phase).toBe("icon");
    }

    act(() => { vi.advanceTimersByTime(350); });
    expect(result.current.transitionState?.kind).toBe("background");
    if (result.current.transitionState?.kind === "background") {
      expect(result.current.transitionState.phase).toBe("expand");
    }

    act(() => { vi.advanceTimersByTime(1430); });
    expect(result.current.transitionState).toBeNull();
    expect(onComplete).toHaveBeenCalled();
  });

  it("playThemeTransition 完整流程：expand → finalize", () => {
    const { result } = renderHook(() => useThemeTransition());
    const onComplete = vi.fn();
    const theme = { id: "violet", label: "紫色", light: { primary: "#7c3aed", accent: "#5b21b6", ring: "#7c3aed" }, dark: { primary: "#a78bfa", accent: "#7c3aed", ring: "#a78bfa" } };

    act(() => {
      result.current.playThemeTransition(theme, "dark", true, "palette", onComplete);
    });

    expect(result.current.transitionState?.kind).toBe("theme");
    if (result.current.transitionState?.kind === "theme") {
      expect(result.current.transitionState.phase).toBe("expand");
      expect(result.current.transitionState.color).toBe("#a78bfa");
      expect(result.current.transitionState.origin).toBe("palette");
    }

    act(() => { vi.advanceTimersByTime(1080); });
    expect(result.current.transitionState).toBeNull();
    expect(onComplete).toHaveBeenCalled();
  });

  it("handleRevealEnd 在非 busy 时无操作", () => {
    const { result } = renderHook(() => useThemeTransition());
    act(() => { result.current.handleRevealEnd(); });
  });
});
