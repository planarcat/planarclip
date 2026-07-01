import {
  DEFAULT_DEVICE_NAME,
  DEFAULT_UI_SETTINGS,
  PREVIEW_UI_SETTINGS_KEY,
  getThemeById,
  normalizeColorScheme,
} from "../constants/theme";
import type { ColorScheme, ThemeColor, UiSettingsPayload } from "../types";

function previewDefaultDeviceName() {
  if (typeof window === "undefined") {
    return DEFAULT_DEVICE_NAME;
  }

  const hostName = window.location.hostname
    .trim()
    .replace(/\.local$/i, "")
    .slice(0, 24);

  return hostName || DEFAULT_DEVICE_NAME;
}

export function normalizeDeviceName(value?: string) {
  const trimmed = value?.trim();
  if (!trimmed || trimmed === DEFAULT_DEVICE_NAME || trimmed.toLowerCase() === "my device") {
    return previewDefaultDeviceName();
  }
  return trimmed;
}

export function loadPreviewUiSettings(): UiSettingsPayload {
  if (typeof window === "undefined") {
    return DEFAULT_UI_SETTINGS;
  }

  try {
    const raw = window.localStorage.getItem(PREVIEW_UI_SETTINGS_KEY);
    if (!raw) {
      return {
        ...DEFAULT_UI_SETTINGS,
        device_name: previewDefaultDeviceName(),
      };
    }

    const parsed = JSON.parse(raw) as Partial<UiSettingsPayload>;
    return {
      color_scheme: normalizeColorScheme(parsed.color_scheme),
      theme_color: getThemeById(parsed.theme_color).id,
      device_name: normalizeDeviceName(parsed.device_name),
    };
  } catch {
    return {
      ...DEFAULT_UI_SETTINGS,
      device_name: previewDefaultDeviceName(),
    };
  }
}

export function savePreviewUiSettings(settings: UiSettingsPayload) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(
      PREVIEW_UI_SETTINGS_KEY,
      JSON.stringify({
        ...settings,
        device_name: normalizeDeviceName(settings.device_name),
      }),
    );
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
