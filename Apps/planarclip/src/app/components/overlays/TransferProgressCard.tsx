import type { TransferProgressState } from "../../hooks/useTransferProgress";
import { formatTransferBytes } from "../../utils/transfer";
import { BottomRightStatusCard } from "./BottomRightStatusCard";
import { ProgressTrack } from "./ProgressTrack";

type TransferProgressCardProps = {
  progress: TransferProgressState;
  onDismiss?: () => void;
};

export function TransferProgressCard({ progress, onDismiss }: TransferProgressCardProps) {
  const hasDeterminateProgress = progress.progress != null;
  const subtitle =
    hasDeterminateProgress &&
    progress.bytesDone != null &&
    progress.bytesTotal != null &&
    progress.bytesTotal > 0
      ? `${formatTransferBytes(progress.bytesDone)} / ${formatTransferBytes(progress.bytesTotal)}`
      : undefined;

  return (
    <BottomRightStatusCard
      title={progress.message}
      subtitle={subtitle}
      onDismiss={!progress.active ? onDismiss : undefined}
      track={
        <ProgressTrack
          mode={hasDeterminateProgress ? "determinate" : "indeterminate"}
          value={progress.progress ?? 0}
        />
      }
    />
  );
}
