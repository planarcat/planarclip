import { X } from "lucide-react";
import { useEffect, useState } from "react";
import { useOverlayLifecycle } from "../../hooks/useOverlayLifecycle";
import { IconButton } from "../ui/IconButton";
import { OverlayPortal } from "../ui/OverlayPortal";

type StatusNoticeProps = {
  open: boolean;
  message: string;
  onDismiss: () => void;
};

export function StatusNotice({ open, message, onDismiss }: StatusNoticeProps) {
  const { mounted, exiting } = useOverlayLifecycle(open);
  const [visibleMessage, setVisibleMessage] = useState(message);

  useEffect(() => {
    if (open && message) {
      setVisibleMessage(message);
    }
  }, [open, message]);

  useEffect(() => {
    if (!open || !message) {
      return;
    }

    const timer = window.setTimeout(onDismiss, 4000);
    return () => window.clearTimeout(timer);
  }, [open, message, onDismiss]);

  if (!mounted) {
    return null;
  }

  const animClass = exiting ? "notice-exit" : "notice-enter";

  return (
    <OverlayPortal>
      <div
        aria-live="polite"
        className="pointer-events-none fixed inset-x-0 bottom-6 z-[80] flex justify-center px-4"
      >
        <div
          role="status"
          className={`pointer-events-auto flex w-full max-w-sm items-start gap-3 overflow-hidden rounded-2xl border border-border bg-card px-5 py-4 shadow-2xl ${animClass}`}
        >
          <p className="min-w-0 flex-1 text-sm font-medium leading-6 text-foreground">{visibleMessage}</p>
          <IconButton onClick={onDismiss} aria-label="关闭提示" title="关闭提示">
            <X size={15} />
          </IconButton>
        </div>
      </div>
    </OverlayPortal>
  );
}
