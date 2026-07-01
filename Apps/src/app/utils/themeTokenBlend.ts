import type { ColorScheme, ThemeColor } from "../types";
import { isColorSchemeDark, themeTransitionEase } from "./themeTransition";

export type ThemeTokenMap = Record<string, string>;

/** CSS variables interpolated during theme / scheme transitions (aligned with index.css). */
export const THEME_BLEND_KEYS = [
  "--background",
  "--foreground",
  "--card",
  "--card-foreground",
  "--popover",
  "--popover-foreground",
  "--secondary",
  "--secondary-foreground",
  "--muted",
  "--muted-foreground",
  "--accent",
  "--accent-foreground",
  "--primary",
  "--primary-foreground",
  "--destructive",
  "--destructive-foreground",
  "--border",
  "--input",
  "--input-background",
  "--switch-background",
  "--ring",
  "--sidebar",
  "--sidebar-foreground",
  "--sidebar-primary",
  "--sidebar-primary-foreground",
  "--sidebar-accent",
  "--sidebar-accent-foreground",
  "--sidebar-border",
  "--sidebar-ring",
] as const;

const LIGHT_SURFACE: ThemeTokenMap = {
  "--background": "#f0f4f9",
  "--foreground": "#0d1117",
  "--card": "#ffffff",
  "--card-foreground": "#0d1117",
  "--popover": "#ffffff",
  "--popover-foreground": "#0d1117",
  "--secondary": "#e4eaf3",
  "--secondary-foreground": "#334155",
  "--muted": "#e8edf5",
  "--muted-foreground": "#55657d",
  "--destructive": "#dc2626",
  "--destructive-foreground": "#ffffff",
  "--border": "rgba(0, 0, 0, 0.08)",
  "--input": "#e4eaf3",
  "--input-background": "#e4eaf3",
  "--switch-background": "#cbd5e1",
  "--sidebar": "#fafafa",
  "--sidebar-foreground": "#252525",
  "--sidebar-primary": "#030213",
  "--sidebar-primary-foreground": "#fafafa",
  "--sidebar-accent": "#f7f7f7",
  "--sidebar-accent-foreground": "#343434",
  "--sidebar-border": "#ebebeb",
  "--sidebar-ring": "#b5b5b5",
};

const DARK_SURFACE: ThemeTokenMap = {
  "--background": "#090c14",
  "--foreground": "#e8eaf0",
  "--card": "#0f1220",
  "--card-foreground": "#e8eaf0",
  "--popover": "#0f1220",
  "--popover-foreground": "#e8eaf0",
  "--secondary": "#161b2e",
  "--secondary-foreground": "#a8b0c8",
  "--muted": "#141826",
  "--muted-foreground": "#7a88a7",
  "--destructive": "#ef4444",
  "--destructive-foreground": "#ffffff",
  "--border": "rgba(255, 255, 255, 0.07)",
  "--input": "#161b2e",
  "--input-background": "#161b2e",
  "--switch-background": "#2d3654",
  "--sidebar": "#0f1220",
  "--sidebar-foreground": "#e8eaf0",
  "--sidebar-accent": "#161b2e",
  "--sidebar-accent-foreground": "#e8eaf0",
  "--sidebar-border": "rgba(255, 255, 255, 0.07)",
};

type Rgba = { r: number; g: number; b: number; a: number };

function clampByte(value: number) {
  return Math.max(0, Math.min(255, Math.round(value)));
}

function parseCssColor(input: string): Rgba | null {
  const value = input.trim();
  if (!value) {
    return null;
  }

  if (value.startsWith("#")) {
    const hex = value.slice(1);
    if (hex.length === 3) {
      return {
        r: parseInt(hex[0] + hex[0], 16),
        g: parseInt(hex[1] + hex[1], 16),
        b: parseInt(hex[2] + hex[2], 16),
        a: 1,
      };
    }
    if (hex.length === 6) {
      return {
        r: parseInt(hex.slice(0, 2), 16),
        g: parseInt(hex.slice(2, 4), 16),
        b: parseInt(hex.slice(4, 6), 16),
        a: 1,
      };
    }
    return null;
  }

  const rgbMatch = value.match(/^rgba?\(([^)]+)\)$/i);
  if (rgbMatch) {
    const parts = rgbMatch[1].split(",").map((part) => part.trim());
    if (parts.length < 3) {
      return null;
    }
    const r = Number(parts[0]);
    const g = Number(parts[1]);
    const b = Number(parts[2]);
    const a = parts[3] !== undefined ? Number(parts[3]) : 1;
    if ([r, g, b, a].some((n) => Number.isNaN(n))) {
      return null;
    }
    return { r, g, b, a };
  }

  return null;
}

function rgbaToCss({ r, g, b, a }: Rgba) {
  if (a >= 0.999) {
    return `rgb(${clampByte(r)}, ${clampByte(g)}, ${clampByte(b)})`;
  }
  return `rgba(${clampByte(r)}, ${clampByte(g)}, ${clampByte(b)}, ${Math.max(0, Math.min(1, a))})`;
}

function lerpCssColor(from: string, to: string, t: number) {
  const a = parseCssColor(from);
  const b = parseCssColor(to);
  if (!a || !b) {
    return t >= 0.5 ? to : from;
  }
  return rgbaToCss({
    r: a.r + (b.r - a.r) * t,
    g: a.g + (b.g - a.g) * t,
    b: a.b + (b.b - a.b) * t,
    a: a.a + (b.a - a.a) * t,
  });
}

export function captureThemeTokens(): ThemeTokenMap {
  const style = getComputedStyle(document.documentElement);
  const tokens: ThemeTokenMap = {};
  for (const key of THEME_BLEND_KEYS) {
    tokens[key] = style.getPropertyValue(key).trim();
  }
  return tokens;
}

export function resolveAppearanceTokens(scheme: ColorScheme, theme: ThemeColor): ThemeTokenMap {
  const dark = isColorSchemeDark(scheme);
  const surface = dark ? DARK_SURFACE : LIGHT_SURFACE;
  const palette = dark ? theme.dark : theme.light;

  return {
    ...surface,
    "--primary": palette.primary,
    "--accent": palette.accent,
    "--ring": palette.ring,
    "--primary-foreground": dark ? "#030b10" : "#ffffff",
    "--sidebar-primary": dark ? palette.primary : surface["--sidebar-primary"] ?? "#030213",
    "--sidebar-ring": dark ? palette.primary : surface["--sidebar-ring"] ?? "#b5b5b5",
  };
}

function applyThemeTokenMap(tokens: ThemeTokenMap) {
  const root = document.documentElement;
  for (const key of THEME_BLEND_KEYS) {
    const value = tokens[key];
    if (value) {
      root.style.setProperty(key, value);
    }
  }
}

export function clearThemeTokenInlineOverrides() {
  const root = document.documentElement;
  for (const key of THEME_BLEND_KEYS) {
    root.style.removeProperty(key);
  }
}

export function blendThemeTokenMaps(
  from: ThemeTokenMap,
  to: ThemeTokenMap,
  t: number,
  options?: { perceptualSurfaceMix?: boolean },
): ThemeTokenMap {
  const perceptual = options?.perceptualSurfaceMix ?? false;
  const oklabMixKeys = new Set([
    "--background",
    "--card",
    "--secondary",
    "--muted",
    "--popover",
    "--input",
    "--input-background",
    "--sidebar",
    "--sidebar-accent",
  ]);

  const blended: ThemeTokenMap = {};
  for (const key of THEME_BLEND_KEYS) {
    const start = from[key] ?? to[key] ?? "";
    const end = to[key] ?? start;
    if (perceptual && oklabMixKeys.has(key) && start && end) {
      const fromPct = Math.max(0, Math.min(100, Math.round((1 - t) * 100)));
      blended[key] = `color-mix(in oklab, ${start} ${fromPct}%, ${end})`;
      continue;
    }
    blended[key] = lerpCssColor(start, end, t);
  }
  return blended;
}

export function runThemeTokenBlend(options: {
  from: ThemeTokenMap;
  to: ThemeTokenMap;
  durationMs: number;
  /** When true, color progress tracks wall-clock linearly (100% dark → 100% light). */
  linearColorProgress?: boolean;
  onDone?: () => void;
}) {
  const { from, to, durationMs, linearColorProgress = false, onDone } = options;
  const startAt = performance.now();
  let frameId = 0;

  const tick = (now: number) => {
    const linear = Math.min(1, (now - startAt) / durationMs);
    const colorT = linearColorProgress ? linear : themeTransitionEase(linear);
    applyThemeTokenMap(
      blendThemeTokenMaps(from, to, colorT, {
        perceptualSurfaceMix: linearColorProgress,
      }),
    );
    if (linear < 1) {
      frameId = requestAnimationFrame(tick);
      return;
    }
    onDone?.();
  };

  frameId = requestAnimationFrame(tick);

  return () => {
    cancelAnimationFrame(frameId);
  };
}
