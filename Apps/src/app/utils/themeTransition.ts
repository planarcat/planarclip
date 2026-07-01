import type { ColorScheme } from "../types";

export type ThemePickOrigin = {
  x: number;
  y: number;
};

export const THEME_TRANSITION_LIGHT_BG = "#f0f4f9";
export const THEME_TRANSITION_DARK_BG = "#090c14";

export const THEME_TRANSITION_ICON_MS = 350;
export const THEME_TRANSITION_ICON_EXIT_MS = 350;
export const THEME_TRANSITION_EXPAND_MS = 1000;

export const THEME_TRANSITION_CIRCLE_REVEAL_ANIMATION = "theme-transition-circle-reveal";
export const THEME_TRANSITION_ICON_EXIT_ANIMATION = "theme-transition-icon-exit";

export function themeTransitionTopRightOrigin(): ThemePickOrigin {
  return { x: window.innerWidth, y: 0 };
}

export function isColorSchemeDark(scheme: ColorScheme): boolean {
  if (scheme === "dark") {
    return true;
  }
  if (scheme === "light") {
    return false;
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Same progress curve as theme-transition.css: cubic-bezier(0.4, 0, 0.2, 1). */
export function themeTransitionEase(t: number): number {
  return cubicBezierY(Math.max(0, Math.min(1, t)), 0.4, 0, 0.2, 1);
}

function cubicBezierY(x: number, x1: number, y1: number, x2: number, y2: number): number {
  let start = 0;
  let end = 1;
  for (let i = 0; i < 12; i += 1) {
    const mid = (start + end) / 2;
    const xAtMid = cubicBezierComponent(mid, x1, x2);
    if (xAtMid < x) {
      start = mid;
    } else {
      end = mid;
    }
  }
  const t = (start + end) / 2;
  return cubicBezierComponent(t, y1, y2);
}

function cubicBezierComponent(t: number, a: number, b: number) {
  const inv = 1 - t;
  return 3 * inv * inv * t * a + 3 * inv * t * t * b + t * t * t;
}

/** Scale ≥1 so a 200vmax-radius circle from `origin` covers the full viewport. */
export function themeTransitionCoverScale(originX: number, originY: number): number {
  const w = window.innerWidth;
  const h = window.innerHeight;
  const maxDist = Math.hypot(Math.max(originX, w - originX), Math.max(originY, h - originY));
  const circleRadius = 200 * (Math.max(w, h) / 100);
  return Math.max(1, (maxDist / circleRadius) * 1.08);
}
