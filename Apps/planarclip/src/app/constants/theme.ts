import type { ColorScheme, ThemeColor, UiSettingsPayload } from "../types";

export const THEME_COLORS: ThemeColor[] = [
  {
    id: "cyan",
    label: "青色",
    dark: { primary: "#22d3ee", accent: "#0e7490", ring: "#22d3ee" },
    light: { primary: "#0891b2", accent: "#0c4a6e", ring: "#0891b2" },
  },
  {
    id: "violet",
    label: "紫色",
    dark: { primary: "#a78bfa", accent: "#7c3aed", ring: "#a78bfa" },
    light: { primary: "#7c3aed", accent: "#5b21b6", ring: "#7c3aed" },
  },
  {
    id: "emerald",
    label: "绿色",
    dark: { primary: "#34d399", accent: "#059669", ring: "#34d399" },
    light: { primary: "#059669", accent: "#065f46", ring: "#059669" },
  },
  {
    id: "rose",
    label: "玫红",
    dark: { primary: "#fb7185", accent: "#e11d48", ring: "#fb7185" },
    light: { primary: "#e11d48", accent: "#9f1239", ring: "#e11d48" },
  },
];

export const DEFAULT_UI_SETTINGS: UiSettingsPayload = {
  color_scheme: "dark",
  theme_color: THEME_COLORS[0].id,
};

export const PREVIEW_UI_SETTINGS_KEY = "planarclip-ui-settings";

export function normalizeColorScheme(value?: string): ColorScheme {
  if (value === "light" || value === "dark" || value === "system") {
    return value;
  }

  return DEFAULT_UI_SETTINGS.color_scheme;
}

export function getThemeById(themeId?: string) {
  return THEME_COLORS.find((theme) => theme.id === themeId) ?? THEME_COLORS[0];
}
