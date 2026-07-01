import type { ViewMode } from "../types";

export const CLIPBOARD_HISTORY_LIMIT_OPTIONS = [25, 50, 100, 200, 500] as const;

export const DEFAULT_CLIPBOARD_HISTORY_LIMIT = 100;

export const DEFAULT_CLIPBOARD_VIEW_MODE: ViewMode = "grid";

export const PREVIEW_CLIPBOARD_VIEW_MODE_KEY = "planarclip_clipboard_view_mode";

export type ClipboardHistoryLimit = (typeof CLIPBOARD_HISTORY_LIMIT_OPTIONS)[number];

export function normalizeClipboardViewMode(value: unknown): ViewMode {
  return value === "list" ? "list" : "grid";
}

export function loadPreviewClipboardViewMode(): ViewMode {
  if (typeof window === "undefined") {
    return DEFAULT_CLIPBOARD_VIEW_MODE;
  }

  try {
    const raw = window.localStorage.getItem(PREVIEW_CLIPBOARD_VIEW_MODE_KEY);
    return normalizeClipboardViewMode(raw);
  } catch {
    return DEFAULT_CLIPBOARD_VIEW_MODE;
  }
}

export function savePreviewClipboardViewMode(mode: ViewMode) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(PREVIEW_CLIPBOARD_VIEW_MODE_KEY, mode);
  } catch {
    // Ignore preview storage failures.
  }
}
