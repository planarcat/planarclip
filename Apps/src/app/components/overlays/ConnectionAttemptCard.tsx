import { BottomRightStatusCard } from "./BottomRightStatusCard";
import { ProgressTrack } from "./ProgressTrack";

type ConnectionAttemptCardProps = {
  deviceName: string;
};

export function ConnectionAttemptCard({ deviceName }: ConnectionAttemptCardProps) {
  const trimmed = deviceName.trim();

  return (
    <BottomRightStatusCard
      title={trimmed ? `正在尝试连接 ${trimmed}` : "正在尝试连接…"}
      track={<ProgressTrack mode="indeterminate" />}
      anchored={false}
    />
  );
}
