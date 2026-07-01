/** Matches Rust `INLINE_IMAGE_BYTES` — transfers at or below this skip the progress card. */
export const TRANSFER_PROGRESS_MIN_BYTES = 512 * 1024;

export function shouldShowTransferProgressCard(payload: {
  active: boolean;
  bytes_total?: number;
  progress?: number | null;
}): boolean {
  if (!payload.active) {
    return false;
  }

  const total = payload.bytes_total;
  if (total != null && total > 0) {
    return total > TRANSFER_PROGRESS_MIN_BYTES;
  }

  return payload.progress != null;
}

export function formatTransferBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB"] as const;
  let value = bytes;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const precision = value >= 100 || unitIndex === 0 ? 0 : 1;
  return `${value.toFixed(precision)} ${units[unitIndex]}`;
}
