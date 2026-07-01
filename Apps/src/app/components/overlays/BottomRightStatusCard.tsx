import type { ReactNode } from "react";
import { X } from "lucide-react";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";

type BottomRightStatusCardProps = {
  title: string;
  subtitle?: string;
  track: ReactNode;
  onDismiss?: () => void;
  anchored?: boolean;
};

export function BottomRightStatusCard({
  title,
  subtitle,
  track,
  onDismiss,
  anchored = true,
}: BottomRightStatusCardProps) {
  const positionClass = anchored
    ? "fixed right-6 bottom-6 z-[70] pointer-events-none"
    : "w-full max-w-[320px] pointer-events-auto";

  return (
    <div aria-live="polite" className={`${positionClass} w-full max-w-[320px]`}>
      <div
        role="status"
        className={`overflow-hidden rounded-2xl border border-border bg-card px-4 py-3.5 shadow-2xl ${onDismiss || !anchored ? "pointer-events-auto" : "pointer-events-none"}`}
      >
        <div className="flex items-start gap-2">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium leading-6 text-foreground">{title}</p>
            {subtitle ? (
              <p className="mt-0.5 text-xs font-medium text-muted-foreground">{subtitle}</p>
            ) : null}
          </div>
          {onDismiss ? (
            <button
              onClick={onDismiss}
              aria-label="关闭"
              className={`shrink-0 rounded-lg p-1.5 text-secondary-foreground ${SURFACE_REVEAL_BG} hover:bg-secondary hover:text-foreground`}
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
