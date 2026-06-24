import { Loader2, ShieldCheck, Wifi, WifiOff, X } from "lucide-react";
import type { Device, PairingStage } from "../../types";
import { OsIcon } from "../common/OsIcon";

type PairingModalProps = {
  pairingCode: string;
  input: string;
  stage: PairingStage;
  discoveredDevices: Device[];
  helperText: string;
  errorMessage: string | null;
  onClose: () => void;
  onInputChange: (value: string) => void;
  onManualPair: () => void;
  onConnectLan: (device: Device) => void;
  onSubmitPairingCode: () => void;
};

export function PairingModal({
  pairingCode,
  input,
  stage,
  discoveredDevices,
  helperText,
  errorMessage,
  onClose,
  onInputChange,
  onManualPair,
  onConnectLan,
  onSubmitPairingCode,
}: PairingModalProps) {
  const statusConfig =
    stage === "manual_pairing" || stage === "requesting_device" || stage === "submitting_code"
      ? {
          icon: <Loader2 size={14} className="shrink-0 animate-spin text-primary" />,
          text: "正在建立连接…",
          className: "text-primary",
        }
      : stage === "awaiting_code"
        ? {
            icon: <ShieldCheck size={14} className="shrink-0 text-amber-500" />,
            text: "等待输入配对码",
            className: "text-amber-500",
          }
        : stage === "incoming_pairing"
          ? {
              icon: <ShieldCheck size={14} className="shrink-0 text-amber-500" />,
              text: "陌生设备请求连接",
              className: "text-amber-500",
            }
          : stage === "incoming_request" || stage === "incoming_accepting"
          ? {
              icon: <Wifi size={14} className="shrink-0 text-primary" />,
              text: stage === "incoming_accepting" ? "正在建立连接…" : "收到新的连接请求",
              className: "text-primary",
            }
          : stage === "error"
            ? {
                icon: <WifiOff size={14} className="shrink-0 text-destructive" />,
                text: errorMessage ?? "连接未完成，请重新尝试。",
                className: "text-destructive",
              }
            : {
                icon: <WifiOff size={14} className="shrink-0 text-muted-foreground" />,
                text: "未连接",
                className: "text-muted-foreground",
              };

  const submitting =
    stage === "manual_pairing" || stage === "requesting_device" || stage === "submitting_code";
  const inboundPairing = stage === "incoming_pairing" || stage === "incoming_accepting";
  const submitLabel =
    stage === "submitting_code"
      ? "正在提交…"
      : stage === "awaiting_code"
        ? "开始配对"
        : stage === "manual_pairing"
          ? "正在校验…"
          : "发起配对";
  const deviceActionLabel = stage === "requesting_device" ? "请求中…" : "连接设备";
  const closeLabel = submitting || inboundPairing ? "取消这次连接" : "关闭";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative mx-4 w-full max-w-sm overflow-hidden rounded-2xl border border-border bg-card shadow-2xl">
        <div className="flex items-center justify-between border-b border-border px-5 pb-4 pt-5">
          <div>
            <p className="text-sm font-semibold text-foreground">
              {stage === "incoming_pairing" ? "收到配对请求" : "连接新设备"}
            </p>
            <p className="mt-0.5 text-[13px] font-medium text-muted-foreground">
              {stage === "incoming_pairing"
                ? "请让对方输入下方配对码以建立连接"
                : "通过配对码与设备列表建立连接"}
            </p>
          </div>
          <button
            onClick={onClose}
            title={closeLabel}
            className="rounded-lg p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-foreground"
            type="button"
          >
            <X size={15} />
          </button>
        </div>

        <div className="max-h-[80vh] space-y-5 overflow-y-auto p-5">
          <div className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3">
            {statusConfig.icon}
            <span className={`text-[13px] font-medium ${statusConfig.className}`}>{statusConfig.text}</span>
          </div>

          <div className="text-center">
            <p className="mb-3 text-[13px] font-medium text-muted-foreground">你的配对码</p>
            <div className="mb-3 flex items-center justify-center gap-2">
              {pairingCode.split("").map((digit, index) => (
                <span
                  key={`${digit}-${index}`}
                  className="flex h-12 w-10 items-center justify-center rounded-lg border border-border bg-secondary font-mono text-2xl font-bold text-primary"
                >
                  {digit}
                </span>
              ))}
            </div>
            <p className="text-[13px] font-medium text-muted-foreground">请在另一台设备上输入此配对码</p>
          </div>

          <div className="flex items-center gap-3">
            <div className="h-px flex-1 bg-border" />
            <span className="text-[13px] font-medium text-muted-foreground">或</span>
            <div className="h-px flex-1 bg-border" />
          </div>

          <div>
            <p className="mb-2 text-[13px] font-medium text-muted-foreground">输入对方设备上的配对码</p>
            <div className="flex gap-2">
              <input
                type="text"
                inputMode="numeric"
                maxLength={6}
                placeholder="000000"
                value={input}
                onChange={(event) => onInputChange(event.target.value.replace(/\D/g, "").slice(0, 6))}
                disabled={submitting || inboundPairing}
                className="flex-1 rounded-lg border border-border bg-secondary px-3 py-2.5 text-center font-mono text-lg tracking-[0.2em] text-foreground transition-colors placeholder:text-muted-foreground/60 focus:border-primary focus:outline-none disabled:opacity-50"
              />
              <button
                onClick={stage === "awaiting_code" ? onSubmitPairingCode : onManualPair}
                disabled={input.length !== 6 || submitting}
                className="rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                type="button"
              >
                {submitting ? <Loader2 size={15} className="animate-spin" /> : submitLabel}
              </button>
            </div>
            <p className={`mt-2 text-[13px] font-medium ${errorMessage ? "text-destructive" : "text-muted-foreground"}`}>
              {errorMessage ?? helperText}
            </p>
          </div>

          <div>
            <div className="mb-2 flex items-center justify-between">
              <p className="text-[13px] font-medium text-muted-foreground">设备列表</p>
              <span className="flex items-center gap-1 text-[13px] font-medium text-muted-foreground">
                <Wifi size={10} />
                自动发现
              </span>
            </div>
            <div className="space-y-2">
              {discoveredDevices.length > 0 ? (
                discoveredDevices.map((device) => (
                  <div
                    key={device.id}
                    className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3 transition-colors hover:border-primary/30"
                  >
                    <div className="rounded-lg bg-primary/10 p-1.5 text-primary">
                      <OsIcon os={device.os} size={14} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <p className="text-[13px] font-medium text-foreground">{device.name}</p>
                      <p className="font-mono text-[13px] font-medium text-secondary-foreground">{device.address}</p>
                    </div>
                    <button
                      onClick={() => onConnectLan(device)}
                      disabled={submitting || inboundPairing}
                      className="shrink-0 rounded-lg bg-primary px-3 py-1.5 text-[13px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                      type="button"
                    >
                      {deviceActionLabel}
                    </button>
                  </div>
                ))
              ) : (
                <div className="rounded-xl border border-dashed border-border px-3 py-5 text-center text-[13px] font-medium text-muted-foreground">
                  暂无发现更多设备
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
