export const CLIPBOARD_HISTORY_LIMIT_OPTIONS = [25, 50, 100, 200, 500] as const;

export const DEFAULT_CLIPBOARD_HISTORY_LIMIT = 100;

export type ClipboardHistoryLimit = (typeof CLIPBOARD_HISTORY_LIMIT_OPTIONS)[number];
