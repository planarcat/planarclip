import { DEFAULT_UI_SETTINGS, PREVIEW_UI_SETTINGS_KEY, getThemeById, normalizeColorScheme } from "../constants/theme";
import type { ColorScheme, ThemeColor, UiSettingsPayload } from "../types";

export function loadPreviewUiSettings(): UiSettingsPayload {
  if (typeof window === "undefined") {
    return DEFAULT_UI_SETTINGS;
  }

  try {
    const raw = window.localStorage.getItem(PREVIEW_UI_SETTINGS_KEY);
    if (!raw) {
      return DEFAULT_UI_SETTINGS;
    }

    const parsed = JSON.parse(raw) as Partial<UiSettingsPayload>;
    return {
      color_scheme: normalizeColorScheme(parsed.color_scheme),
      theme_color: getThemeById(parsed.theme_color).id,
    };
  } catch {
    return DEFAULT_UI_SETTINGS;
  }
}

export function savePreviewUiSettings(settings: UiSettingsPayload) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(PREVIEW_UI_SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // 忽略预览态下的本地存储失败，保持界面继续可用。
  }
}

export function isDarkActive() {
  return document.documentElement.classList.contains("dark");
}

export function applyColorScheme(scheme: ColorScheme) {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.classList.toggle(
    "dark",
    scheme === "dark" || (scheme === "system" && prefersDark),
  );
}

export function applyThemeColor(theme: ThemeColor) {
  const selected = isDarkActive() ? theme.dark : theme.light;
  const root = document.documentElement;
  root.style.setProperty("--primary", selected.primary);
  root.style.setProperty("--accent", selected.accent);
  root.style.setProperty("--ring", selected.ring);
  root.style.setProperty("--primary-foreground", isDarkActive() ? "#030b10" : "#ffffff");
}
