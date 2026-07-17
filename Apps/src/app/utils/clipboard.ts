import { invoke, isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
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
    mediaRef: item.media_ref ?? undefined,
    fileCount: item.file_count ?? undefined,
    fileNames: item.file_names ?? undefined,
    previewKind: normalizePreviewKind(item.preview_kind),
  }));
}

export async function resolveHistoryMediaUrl(mediaRef: string): Promise<string | undefined> {
  if (!isTauri() || !mediaRef) {
    return undefined;
  }

  try {
    return await invoke<string>("read_history_media", { mediaRef });
  } catch {
    return undefined;
  }
}

/**
 * Lazily load a history media file (image original or content thumbnail, stored
 * under history_media/) into a data URL for display. Returns undefined until loaded.
 */
export function useHistoryMediaUrl(mediaRef?: string): string | undefined {
  const [url, setUrl] = useState<string | undefined>(undefined);

  useEffect(() => {
    if (!mediaRef) {
      setUrl(undefined);
      return;
    }
    let cancelled = false;
    resolveHistoryMediaUrl(mediaRef).then((resolved) => {
      if (!cancelled) {
        setUrl(resolved);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [mediaRef]);

  return url;
}

export async function resolveTypeIconUrl(ext: string): Promise<string | undefined> {
  if (!isTauri() || !ext) {
    return undefined;
  }

  try {
    return await invoke<string>("read_type_icon", { ext });
  } catch {
    return undefined;
  }
}

/**
 * Lazily load a file-type icon (cached per extension under cache/icons/) into a
 * data URL. Same extension reuses one icon. Returns undefined until loaded.
 */
export function useTypeIconUrl(ext?: string): string | undefined {
  const [url, setUrl] = useState<string | undefined>(undefined);

  useEffect(() => {
    if (!ext) {
      setUrl(undefined);
      return;
    }
    let cancelled = false;
    resolveTypeIconUrl(ext).then((resolved) => {
      if (!cancelled) {
        setUrl(resolved);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [ext]);

  return url;
}

export function fileExtension(fileName?: string): string | undefined {
  if (!fileName) {
    return undefined;
  }
  const idx = fileName.lastIndexOf(".");
  if (idx <= 0 || idx === fileName.length - 1) {
    return undefined;
  }
  return fileName.slice(idx + 1).toLowerCase();
}
