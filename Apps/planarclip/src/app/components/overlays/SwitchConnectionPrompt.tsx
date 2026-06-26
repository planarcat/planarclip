import { ArrowLeftRight, X } from "lucide-react";
import type { Device } from "../../types";

type SwitchConnectionPromptProps = {
  currentDeviceName: string;
  targetDevice: Device;
  onConfirm: () => void;
  onCancel: () => void;
};

export function SwitchConnectionPrompt({
  currentDeviceName,
  targetDevice,
  onConfirm,
  onCancel,
}: SwitchConnectionPromptProps) {
  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onCancel} />
      <div
        role="dialog"
        aria-labelledby="switch-connection-title"
        aria-describedby="switch-connection-description"
        className="relative mx-4 w-full max-w-sm overflow-hidden rounded-2xl border border-border bg-card shadow-2xl"
      >
        <div className="border-b border-border px-5 pb-4 pt-5">
          <div className="flex items-start gap-3">
            <div className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-secondary text-muted-foreground">
              <ArrowLeftRight size={18} />
            </div>
            <div className="min-w-0 flex-1">
              <p id="switch-connection-title" className="text-sm font-semibold text-foreground">
                切换连接设备
              </p>
              <p id="switch-connection-description" className="mt-1 text-[13px] font-medium leading-6 text-muted-foreground">
                当前已有配对设备 <span className="text-foreground">{currentDeviceName}</span>，连接{" "}
                <span className="text-foreground">{targetDevice.name}</span> 会取消当前配对，是否继续？
              </p>
            </div>
            <button
              onClick={onCancel}
              aria-label="取消切换"
              className="rounded-lg p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-foreground"
              type="button"
            >
              <X size={15} />
            </button>
          </div>
        </div>

        <div className="flex gap-2 p-5">
          <button
            onClick={onCancel}
            className="flex-1 rounded-lg border border-border bg-secondary px-3 py-2.5 text-sm font-medium text-foreground transition-colors hover:bg-secondary/80"
            type="button"
          >
            取消
          </button>
          <button
            onClick={onConfirm}
            className="flex-1 rounded-lg bg-primary px-3 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
            type="button"
          >
            继续连接
          </button>
        </div>
      </div>
    </div>
  );
}
