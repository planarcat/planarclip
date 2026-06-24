import { Loader2, ShieldCheck, Smartphone, X } from "lucide-react";
import type { ConnectionRequestPayload } from "../../types";

type IncomingConnectionPromptProps = {
  request: ConnectionRequestPayload;
  accepting: boolean;
  onAccept: () => void;
  onReject: () => void;
};

export function IncomingConnectionPrompt({ request, accepting, onAccept, onReject }: IncomingConnectionPromptProps) {
  const isPairing = request.requires_pairing;

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
                {isPairing
                  ? "想要与这台设备配对。确认后请让对方输入本机配对码。"
                  : "想要连接这台设备。请确认是否允许这次连接。"}
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
          <div className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3">
            <ShieldCheck size={14} className="shrink-0 text-primary" />
            <p className="text-[13px] font-medium text-muted-foreground">
              {isPairing
                ? "允许后，对方会收到配对提示，需要输入你屏幕上的 6 位配对码才能完成连接。"
                : "允许后，这台设备会被加入已配对列表，下次在同一局域网内可直接连接。"}
            </p>
          </div>

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
