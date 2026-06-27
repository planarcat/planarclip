type ConnectionAttemptCardProps = {
  deviceName: string;
};

export function ConnectionAttemptCard({ deviceName }: ConnectionAttemptCardProps) {
  const trimmed = deviceName.trim();

  return (
    <div
      aria-live="polite"
      className="pointer-events-none fixed right-6 bottom-6 z-[70] w-full max-w-[320px]"
    >
      <div
        role="status"
        className="overflow-hidden rounded-2xl border border-border bg-card px-4 py-3.5 shadow-2xl"
      >
        <p className="text-sm font-medium leading-6 text-foreground">
          {trimmed ? `正在尝试连接 ${trimmed}` : "正在尝试连接…"}
        </p>
        <div className="connection-attempt-track mt-3.5" aria-hidden="true">
          <div className="connection-attempt-flow">
            <span className="connection-attempt-segment" />
            <span className="connection-attempt-segment" />
          </div>
        </div>
      </div>
    </div>
  );
}
