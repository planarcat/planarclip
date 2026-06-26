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
    <div className="fixed inset-0 z-[70] flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-[2px]" onClick={onDismiss} />
      <div
        role="alertdialog"
        aria-live="assertive"
        className="relative mx-4 w-full max-w-sm overflow-hidden rounded-2xl border border-border bg-card px-5 py-4 shadow-2xl"
      >
        <div className="flex items-start gap-3">
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
    </div>
  );
}
