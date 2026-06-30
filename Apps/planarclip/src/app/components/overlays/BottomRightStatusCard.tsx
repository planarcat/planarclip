import type { ReactNode } from "react";
import { X } from "lucide-react";

type BottomRightStatusCardProps = {
  title: string;
  subtitle?: string;
  track: ReactNode;
  onDismiss?: () => void;
};

export function BottomRightStatusCard({ title, subtitle, track, onDismiss }: BottomRightStatusCardProps) {
  return (
    <div
      aria-live="polite"
      className={`fixed right-6 bottom-6 z-[70] w-full max-w-[320px] ${onDismiss ? "pointer-events-auto" : "pointer-events-none"}`}
    >
      <div
        role="status"
        className="overflow-hidden rounded-2xl border border-border bg-card px-4 py-3.5 shadow-2xl"
      >
        <div className="flex items-start gap-2">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium leading-6 text-foreground">{title}</p>
            {subtitle ? (
              <p className="mt-0.5 text-[12px] font-medium text-muted-foreground">{subtitle}</p>
            ) : null}
          </div>
          {onDismiss ? (
            <button
              onClick={onDismiss}
              aria-label="关闭"
              className="shrink-0 rounded-lg p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-foreground"
              type="button"
            >
              <X size={15} />
            </button>
          ) : null}
        </div>
        {track}
      </div>
    </div>
  );
}
