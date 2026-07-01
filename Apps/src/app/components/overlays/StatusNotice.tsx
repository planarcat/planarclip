import { X } from "lucide-react";
import { useEffect } from "react";

type StatusNoticeProps = {
  message: string;
  onDismiss: () => void;
};

export function StatusNotice({ message, onDismiss }: StatusNoticeProps) {
  useEffect(() => {
    const timer = window.setTimeout(onDismiss, 4000);
    return () => window.clearTimeout(timer);
  }, [message, onDismiss]);

  return (
    <div
      aria-live="polite"
      className="pointer-events-none fixed inset-x-0 bottom-6 z-[80] flex justify-center px-4"
    >
      <div
        role="status"
        className="pointer-events-auto flex w-full max-w-sm items-start gap-3 overflow-hidden rounded-2xl border border-border bg-card px-5 py-4 shadow-2xl"
      >
        <p className="min-w-0 flex-1 text-sm font-medium leading-6 text-foreground">{message}</p>
        <button
          onClick={onDismiss}
          aria-label="关闭提示"
          className="shrink-0 rounded-lg p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-foreground"
          type="button"
        >
          <X size={15} />
        </button>
      </div>
    </div>
  );
}
