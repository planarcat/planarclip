import { Loader2, Smartphone, X } from "lucide-react";
import { CONNECTION_RESPONSE_TIMEOUT_SECS } from "../../constants/connection";
import { usePairingCountdown } from "../../hooks/usePairingCountdown";
import type { ConnectionRequestPayload } from "../../types";

type IncomingConnectionPromptProps = {
  request: ConnectionRequestPayload;
  accepting: boolean;
  onAccept: () => void;
  onReject: () => void;
  onTimeout: () => void;
};

export function IncomingConnectionPrompt({
  request,
  accepting,
  onAccept,
  onReject,
  onTimeout,
}: IncomingConnectionPromptProps) {
  const isPairing = request.requires_pairing;

  const { remainingSeconds, progress, isUrgent } = usePairingCountdown({
    active: !accepting,
    onExpire: onTimeout,
    durationSecs: CONNECTION_RESPONSE_TIMEOUT_SECS,
  });

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" />
      <div
        role="dialog"
        aria-labelledby="incoming-connection-title"
        aria-describedby="incoming-connection-description"
        className="relative mx-4 w-full max-w-sm overflow-hidden rounded-2xl border border-primary/25 bg-card shadow-2xl"
      >
        <div className="border-b border-border bg-primary/5 px-5 pb-4 pt-5">
          <div className="flex items-start gap-3">
            <div className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary/15 text-primary">
              <Smartphone size={18} />
            </div>
            <div className="min-w-0 flex-1">
              <p id="incoming-connection-title" className="text-sm font-semibold text-foreground">
                {isPairing ? "陌生设备请求配对" : "收到新的连接请求"}
              </p>
              <p id="incoming-connection-description" className="mt-1 text-[13px] font-medium leading-6 text-muted-foreground">
                <span className="text-foreground">{request.device_name}</span>{" "}
                {isPairing ? "想要与这台设备配对，请确认是否允许。" : "想要连接这台设备，请确认是否允许。"}
              </p>
            </div>
            <button
              onClick={onReject}
              disabled={accepting}
              aria-label="拒绝这次连接"
              className="rounded-lg p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-foreground disabled:opacity-40"
              type="button"
            >
              <X size={15} />
            </button>
          </div>
        </div>

        <div className="space-y-4 p-5">
          {!accepting && (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-[11px] font-medium text-muted-foreground">
                <span className={isUrgent ? "text-destructive" : undefined}>
                  {remainingSeconds} 秒内未回应将视为拒绝
                </span>
                <span>{remainingSeconds}s</span>
              </div>
              <div className="h-[3px] w-full overflow-hidden rounded-full bg-secondary">
                <div
                  className={`h-full rounded-full transition-all duration-1000 ease-linear ${
                    isUrgent ? "bg-destructive animate-pulse" : "bg-primary"
                  }`}
                  style={{ width: `${Math.max(0, Math.min(100, progress * 100))}%` }}
                />
              </div>
            </div>
          )}

          {!isPairing && (
            <p className="text-[13px] font-medium text-muted-foreground">
              允许后，这台设备会被加入已配对列表。
            </p>
          )}

          <div className="flex gap-2">
            <button
              onClick={onReject}
              disabled={accepting}
              className="flex-1 rounded-lg border border-border bg-secondary px-3 py-2.5 text-sm font-medium text-foreground transition-colors hover:bg-secondary/80 disabled:opacity-40"
              type="button"
            >
              拒绝
            </button>
            <button
              onClick={onAccept}
              disabled={accepting}
              className="inline-flex flex-1 items-center justify-center gap-2 rounded-lg bg-primary px-3 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
              type="button"
            >
              {accepting ? (
                <>
                  <Loader2 size={15} className="animate-spin" />
                  正在连接…
                </>
              ) : isPairing ? (
                "允许配对"
              ) : (
                "允许连接"
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
