import { useCallback, useEffect, useRef, useState } from "react";
import { shouldShowTransferProgressCard } from "../utils/transfer";

export type TransferProgressState = {
  active: boolean;
  message: string;
  progress: number | null;
  label?: string;
  bytesDone?: number;
  bytesTotal?: number;
  batchIndex?: number;
  batchTotal?: number;
};

export type ClipboardSyncActivityPayload = {
  active: boolean;
  kind: string;
  message: string;
  progress?: number | null;
  direction?: "send" | "receive";
  label?: string;
  bytes_done?: number;
  bytes_total?: number;
  batch_index?: number;
  batch_total?: number;
};

const COMPLETE_DISMISS_MS = 5000;

export function useTransferProgress() {
  const [transferProgress, setTransferProgress] = useState<TransferProgressState | null>(null);
  const dismissTimerRef = useRef<number | null>(null);

  const clearDismissTimer = useCallback(() => {
    if (dismissTimerRef.current != null) {
      window.clearTimeout(dismissTimerRef.current);
      dismissTimerRef.current = null;
    }
  }, []);

  const clearTransferProgress = useCallback(() => {
    clearDismissTimer();
    setTransferProgress(null);
  }, [clearDismissTimer]);

  const applySyncActivity = useCallback(
    (payload: ClipboardSyncActivityPayload) => {
      if (payload.kind === "notice") {
        clearTransferProgress();
        return;
      }

      if (payload.kind !== "file" && payload.kind !== "image") {
        return;
      }

      if (!payload.active) {
        clearDismissTimer();
        setTransferProgress((current) => {
          if (!current) {
            return null;
          }

          return {
            ...current,
            active: false,
            message: payload.message || current.message,
            progress: 1,
          };
        });
        dismissTimerRef.current = window.setTimeout(() => {
          setTransferProgress(null);
          dismissTimerRef.current = null;
        }, COMPLETE_DISMISS_MS);
        return;
      }

      if (!shouldShowTransferProgressCard(payload)) {
        return;
      }

      clearDismissTimer();
      setTransferProgress({
        active: true,
        message: payload.message,
        progress: payload.progress ?? null,
        label: payload.label,
        bytesDone: payload.bytes_done,
        bytesTotal: payload.bytes_total,
        batchIndex: payload.batch_index,
        batchTotal: payload.batch_total,
      });
    },
    [clearDismissTimer, clearTransferProgress],
  );

  useEffect(() => () => clearDismissTimer(), [clearDismissTimer]);

  return {
    transferProgress,
    applySyncActivity,
    clearTransferProgress,
  };
}
