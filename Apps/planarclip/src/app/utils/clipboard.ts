import type { ClipboardHistoryPayload, ClipEntry, ClipType } from "../types";

export function formatClipSize(content: string) {
  const bytes = new TextEncoder().encode(content).length;
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function normalizeClipType(value?: string): ClipType {
  if (value === "image" || value === "file") {
    return value;
  }
  return "text";
}

export function mapClipboardHistory(payload: ClipboardHistoryPayload[]): ClipEntry[] {
  return payload.map((item) => ({
    id: item.id,
    type: normalizeClipType(item.clip_type),
    content: item.content,
    sourceLabel: item.source_label,
    direction: item.direction,
    size: item.size_label ?? formatClipSize(item.content),
    timestamp: new Date(item.timestamp_ms),
    imagePreviewUrl: item.image_data_url ?? undefined,
  }));
}
