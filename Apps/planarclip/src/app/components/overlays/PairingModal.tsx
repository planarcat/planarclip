import { Loader2, PlugZap, Wifi, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { usePairingCountdown } from "../../hooks/usePairingCountdown";
import type { Device, PairingStage } from "../../types";
import { OsIcon } from "../common/OsIcon";

type PairingModalProps = {
  initialTarget?: Device | null;
  allDiscoverable: Device[];
  pairingCode: string;
  input: string;
  stage: PairingStage;
  helperText: string;
  errorMessage: string | null;
  rotationHint: string | null;
  connectionLocked: boolean;
  onClose: () => void;
  onInputChange: (value: string) => void;
  onSelectDevice: (device: Device) => void;
  onSubmitPairingCode: () => void;
  onRotatePairingCode: () => void;
};

function PairingModalHeader({
  stage,
  selectedDevice,
  onClose,
  closeLabel,
}: {
  stage: PairingStage;
  selectedDevice: Device | null;
  onClose: () => void;
  closeLabel: string;
}) {
  const title =
    stage === "incoming_pairing"
      ? "收到配对请求"
      : selectedDevice
        ? `连接 ${selectedDevice.name}`
        : "连接新设备";

  const subtitle =
    stage === "incoming_pairing"
      ? "输入对方配对码，或让对方输入你的配对码"
      : selectedDevice
        ? "选择设备后发起连接，按提示完成配对"
        : "请先从列表中选择要连接的设备";

  return (
    <div className="flex items-center justify-between border-b border-border px-5 pb-4 pt-5">
      <div className="min-w-0 pr-3">
        <p className="truncate text-sm font-semibold text-foreground">{title}</p>
        <p className="mt-0.5 text-[11px] font-medium text-muted-foreground">{subtitle}</p>
      </div>
      <button
        onClick={onClose}
        title={closeLabel}
        className="shrink-0 rounded-lg p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-foreground"
        type="button"
      >
        <X size={15} />
      </button>
    </div>
  );
}

function SelectedDeviceCard({ device }: { device: Device }) {
  return (
    <div className="border-b border-border px-5 pb-4 pt-4">
      <div className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3">
        <div className="rounded-lg bg-secondary p-2 text-muted-foreground">
          <OsIcon os={device.os} size={15} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-[11px] font-medium text-foreground">{device.name}</p>
          <p className="truncate font-mono text-[10px] font-medium text-muted-foreground">{device.address}</p>
        </div>
      </div>
    </div>
  );
}

function PairingStatusBar({ stage }: { stage: PairingStage }) {
  const showStatus =
    stage === "awaiting_code" ||
    stage === "incoming_pairing" ||
    stage === "requesting_device" ||
    stage === "submitting_code";

  if (!showStatus) {
    return null;
  }

  const isBusy = stage === "requesting_device" || stage === "submitting_code";
  const text =
    stage === "requesting_device"
      ? "等待对方回应…"
      : stage === "submitting_code"
        ? "正在验证…"
        : stage === "awaiting_code" || stage === "incoming_pairing"
          ? "请输入配对码"
          : "等待对方输入配对码";

  return (
    <div className="flex items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3">
      {isBusy ? (
        <Loader2 size={13} className="shrink-0 animate-spin text-muted-foreground" />
      ) : (
        <Wifi size={13} className="shrink-0 text-muted-foreground" />
      )}
      <span className="text-[11px] font-medium text-muted-foreground">{text}</span>
    </div>
  );
}

function LocalPairingCodeSection({
  pairingCode,
  isUrgent,
  progress,
}: {
  pairingCode: string;
  isUrgent: boolean;
  progress: number;
}) {
  const digits = pairingCode.padEnd(6, "•").slice(0, 6).split("");
  const digitClassName = isUrgent
    ? "border-destructive/40 text-destructive animate-pulse"
    : "border-border text-primary";
  const barClassName = isUrgent ? "bg-destructive animate-pulse" : "bg-primary";

  return (
    <div className="text-center">
      <p className="mb-3 text-[11px] font-medium text-muted-foreground">本机配对码</p>
      <div className="mb-3 flex items-center justify-center gap-1.5">
        {digits.map((digit, index) => (
          <span
            key={`${digit}-${index}`}
            className={`flex h-11 w-9 items-center justify-center rounded-lg border bg-secondary font-mono text-xl font-bold ${digitClassName}`}
          >
            {digit}
          </span>
        ))}
      </div>
      <div className="mx-auto mb-3 h-[3px] w-full overflow-hidden rounded-full bg-secondary">
        <div
          className={`h-full rounded-full transition-all duration-1000 ease-linear ${barClassName}`}
          style={{ width: `${Math.max(0, Math.min(100, progress * 100))}%` }}
        />
      </div>
      <p className="text-[11px] font-medium text-muted-foreground">本机配对码 · 也可输入对方配对码</p>
    </div>
  );
}

function SelectDeviceHint() {
  return (
    <div className="rounded-xl border border-dashed border-border bg-secondary/20 px-4 py-6 text-center">
      <p className="text-[13px] font-medium text-foreground">请先从下方列表选择要连接的设备</p>
      <p className="mt-1 text-[11px] font-medium text-muted-foreground">选中后将向该设备发起连接请求</p>
    </div>
  );
}

function SwitchableDeviceList({
  devices,
  disabled,
  onSelectDevice,
}: {
  devices: Device[];
  disabled: boolean;
  onSelectDevice: (device: Device) => void;
}) {
  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <p className="text-[11px] font-medium text-foreground">设备列表</p>
        <span className="flex items-center gap-1 text-[11px] font-medium text-muted-foreground">
          <Wifi size={10} />
          自动发现
        </span>
      </div>
      <div className="space-y-2">
        {devices.length > 0 ? (
          devices.map((device) => (
            <button
              key={device.id}
              type="button"
              disabled={disabled}
              onClick={() => onSelectDevice(device)}
              className="flex w-full items-center gap-3 rounded-xl border border-border bg-secondary/30 p-3 text-left transition-colors hover:border-primary/30 disabled:pointer-events-none disabled:opacity-40"
            >
              <div className="rounded-lg bg-secondary p-2 text-muted-foreground">
                <OsIcon os={device.os} size={15} />
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate text-[11px] font-medium text-foreground">{device.name}</p>
                <p className="truncate font-mono text-[10px] font-medium text-muted-foreground">{device.address}</p>
              </div>
              <PlugZap size={14} className="shrink-0 text-muted-foreground" />
            </button>
          ))
        ) : (
          <div className="rounded-xl border border-dashed border-border px-3 py-5 text-center text-[11px] font-medium text-muted-foreground">
            暂无发现更多设备
          </div>
        )}
      </div>
    </div>
  );
}

export function PairingModal({
  initialTarget = null,
  allDiscoverable,
  pairingCode,
  input,
  stage,
  helperText,
  errorMessage,
  rotationHint,
  connectionLocked,
  onClose,
  onInputChange,
  onSelectDevice,
  onSubmitPairingCode,
  onRotatePairingCode,
}: PairingModalProps) {
  const [selectedDevice, setSelectedDevice] = useState<Device | null>(initialTarget ?? null);

  useEffect(() => {
    setSelectedDevice(initialTarget?.name ? initialTarget : null);
  }, [initialTarget]);

  const activeTarget = selectedDevice?.name ? selectedDevice : null;
  const listDevices = allDiscoverable.filter((device) => device.id !== activeTarget?.id);

  const inMutualPairing = stage === "awaiting_code" || stage === "incoming_pairing";
  const showLocalPairingCode = inMutualPairing;
  const showPairingInput = inMutualPairing || stage === "submitting_code";
  const inboundPairing = stage === "incoming_pairing";

  const pairingCodeCountdownActive = stage === "awaiting_code" || stage === "incoming_pairing";
  const { progress, isUrgent } = usePairingCountdown({
    active: pairingCodeCountdownActive,
    onExpire: onRotatePairingCode,
  });

  const selectFromList = useCallback(
    (device: Device) => {
      if (connectionLocked) {
        return;
      }
      setSelectedDevice(device);
      onSelectDevice(device);
    },
    [connectionLocked, onSelectDevice],
  );

  const closeLabel = connectionLocked || inboundPairing ? "取消这次连接" : "关闭";
  const helperLine = errorMessage ?? rotationHint ?? helperText;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative mx-4 w-full max-w-[360px] overflow-hidden rounded-2xl border border-border bg-card shadow-2xl">
        <PairingModalHeader
          stage={stage}
          selectedDevice={activeTarget}
          onClose={onClose}
          closeLabel={closeLabel}
        />

        {activeTarget && !inboundPairing && <SelectedDeviceCard device={activeTarget} />}

        <div className="max-h-[80vh] space-y-5 overflow-y-auto p-5">
          {!activeTarget && <SelectDeviceHint />}

          <PairingStatusBar stage={stage} />

          {showLocalPairingCode && (
            <LocalPairingCodeSection pairingCode={pairingCode} isUrgent={isUrgent} progress={progress} />
          )}

          {showPairingInput && (
            <div>
              <div className="flex gap-2">
                <input
                  type="text"
                  inputMode="numeric"
                  maxLength={6}
                  placeholder="000000"
                  value={input}
                  onChange={(event) => onInputChange(event.target.value.replace(/\D/g, "").slice(0, 6))}
                  disabled={stage === "submitting_code"}
                  className="flex-1 rounded-lg border border-border bg-secondary px-3 py-2.5 text-center font-mono text-base tracking-[0.2em] text-foreground transition-colors placeholder:text-muted-foreground/60 focus:border-primary focus:outline-none disabled:opacity-50"
                />
                <button
                  onClick={onSubmitPairingCode}
                  disabled={input.length !== 6 || stage === "submitting_code"}
                  className="shrink-0 rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
                  type="button"
                >
                  {stage === "submitting_code" ? <Loader2 size={15} className="animate-spin" /> : "验证"}
                </button>
              </div>
              <p
                className={`mt-2 text-[11px] font-medium ${
                  errorMessage ? "text-destructive" : rotationHint ? "text-primary" : "text-muted-foreground"
                }`}
              >
                {helperLine}
              </p>
            </div>
          )}

          {!showPairingInput && !inMutualPairing && (
            <p
              className={`text-[11px] font-medium ${
                errorMessage ? "text-destructive" : rotationHint ? "text-primary" : "text-muted-foreground"
              }`}
            >
              {helperLine}
            </p>
          )}

          {!inboundPairing && (
            <SwitchableDeviceList
              devices={listDevices}
              disabled={connectionLocked}
              onSelectDevice={selectFromList}
            />
          )}
        </div>
      </div>
    </div>
  );
}
