import type { ClipboardHistoryPayload, ClipEntry } from "../types";

export function formatClipSize(content: string) {
  const bytes = new TextEncoder().encode(content).length;
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
}

export function mapClipboardHistory(payload: ClipboardHistoryPayload[]): ClipEntry[] {
  return payload.map((item) => ({
    id: item.id,
    type: "text",
    content: item.content,
    sourceLabel: item.source_label,
    direction: item.direction,
    size: formatClipSize(item.content),
    timestamp: new Date(item.timestamp_ms),
  }));
}
