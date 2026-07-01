import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ClipboardHistoryPayload, ClipEntry } from "../types";

function normalizePreviewKind(value?: string | null): "thumbnail" | "icon" | undefined {
  if (value === "thumbnail" || value === "icon") {
    return value;
  }
  return undefined;
}

export function formatClipSize(content: string) {
  const bytes = new TextEncoder().encode(content).length;
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function normalizeClipType(value?: string): ClipEntry["type"] {
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
    fileCount: item.file_count ?? undefined,
    fileNames: item.file_names ?? undefined,
    previewKind: normalizePreviewKind(item.preview_kind),
    thumbnailRef: item.thumbnail_ref ?? undefined,
  }));
}

export async function resolveHistoryThumbnailUrl(thumbnailRef: string): Promise<string | undefined> {
  if (!isTauri() || !thumbnailRef) {
    return undefined;
  }

  try {
    return await invoke<string>("resolve_history_thumbnail", { thumbnailRef });
  } catch {
    return undefined;
  }
}
