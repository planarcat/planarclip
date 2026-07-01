type ProgressTrackProps = {
  mode: "indeterminate" | "determinate";
  value?: number;
};

export function ProgressTrack({ mode, value = 0 }: ProgressTrackProps) {
  if (mode === "indeterminate") {
    return (
      <div className="connection-attempt-track mt-3.5" aria-hidden="true">
        <div className="connection-attempt-flow">
          <span className="connection-attempt-segment" />
          <span className="connection-attempt-segment" />
        </div>
      </div>
    );
  }

  const pct = Math.max(0, Math.min(100, Math.round(value * 100)));

  return (
    <div className="connection-attempt-track mt-3.5">
      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
        className="h-full rounded-full bg-primary motion-reduce:transition-none transition-all duration-150 ease-out"
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}
