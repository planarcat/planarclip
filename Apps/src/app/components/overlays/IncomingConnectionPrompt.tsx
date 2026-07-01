import { Loader2, Smartphone, X } from "lucide-react";
import { CONNECTION_RESPONSE_TIMEOUT_SECS } from "../../constants/connection";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";
import { usePairingCountdown } from "../../hooks/usePairingCountdown";
import type { ConnectionRequestPayload } from "../../types";
import { IconButton } from "../ui/IconButton";
import { ModalShell } from "../ui/ModalShell";
import { PrimaryButton } from "../ui/PrimaryButton";

type IncomingConnectionPromptProps = {
  open: boolean;
  request: ConnectionRequestPayload;
  accepting: boolean;
  onAccept: () => void;
  onReject: () => void;
  onTimeout: () => void;
};

export function IncomingConnectionPrompt({
  open,
  request,
  accepting,
  onAccept,
  onReject,
  onTimeout,
}: IncomingConnectionPromptProps) {
  const isPairing = request.requires_pairing;

  const { remainingSeconds, progress, isUrgent } = usePairingCountdown({
    active: open && !accepting,
    onExpire: onTimeout,
    durationSecs: CONNECTION_RESPONSE_TIMEOUT_SECS,
  });

  return (
    <ModalShell
      open={open}
      zIndexClassName="z-[60]"
      panelClassName="border-primary/25"
      labelledBy="incoming-connection-title"
      describedBy="incoming-connection-description"
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
            <p id="incoming-connection-description" className="mt-1 text-sm font-medium leading-6 text-muted-foreground">
              <span className="text-foreground">{request.device_name}</span>{" "}
              {isPairing ? "想要与这台设备配对，请确认是否允许。" : "想要连接这台设备，请确认是否允许。"}
            </p>
          </div>
          <IconButton
            onClick={onReject}
            disabled={accepting}
            aria-label="拒绝这次连接"
            title="拒绝这次连接"
          >
            <X size={15} />
          </IconButton>
        </div>
      </div>

      <div className="space-y-4 p-5">
        {!accepting && (
          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs font-medium text-muted-foreground">
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
          <p className="text-sm font-medium text-muted-foreground">允许后，将与此设备建立连接。</p>
        )}

        {isPairing && (
          <p className="text-sm font-medium text-muted-foreground">
            配对成功后，这台设备会加入熟悉列表，并默认信任其来访。
          </p>
        )}

        <div className="flex gap-2">
          <button
            onClick={onReject}
            disabled={accepting}
            className={`flex-1 rounded-lg border border-border bg-secondary px-3 py-2.5 text-sm font-medium text-foreground ${SURFACE_REVEAL_BG} hover:bg-secondary/80 disabled:opacity-40`}
            type="button"
          >
            拒绝
          </button>
          <PrimaryButton
            onClick={onAccept}
            disabled={accepting}
            className="inline-flex flex-1 items-center justify-center gap-2 px-3"
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
          </PrimaryButton>
        </div>
      </div>
    </ModalShell>
  );
}
