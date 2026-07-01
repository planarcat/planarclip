import type { ReactNode } from "react";
import { useOverlayLifecycle } from "../../hooks/useOverlayLifecycle";
import { OverlayPortal } from "./OverlayPortal";

type ModalShellProps = {
  open: boolean;
  onBackdropClick?: () => void;
  zIndexClassName?: string;
  panelClassName?: string;
  labelledBy?: string;
  describedBy?: string;
  children: ReactNode;
};

export function ModalShell({
  open,
  onBackdropClick,
  zIndexClassName = "z-50",
  panelClassName = "",
  labelledBy,
  describedBy,
  children,
}: ModalShellProps) {
  const { mounted, exiting } = useOverlayLifecycle(open);

  if (!mounted) {
    return null;
  }

  const backdropClass = exiting ? "overlay-backdrop-exit" : "overlay-backdrop-enter";
  const panelAnimClass = exiting ? "overlay-panel-exit" : "overlay-panel-enter";

  return (
    <OverlayPortal>
      <div className={`fixed inset-0 flex items-center justify-center ${zIndexClassName}`}>
        <div
          className={`absolute inset-0 bg-black/60 backdrop-blur-sm ${backdropClass}`}
          onClick={onBackdropClick}
          aria-hidden
        />
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby={labelledBy}
          aria-describedby={describedBy}
          className={`relative mx-4 w-full overflow-hidden rounded-2xl border border-border bg-card shadow-2xl ${panelAnimClass} ${panelClassName || "max-w-[380px]"}`}
        >
          {children}
        </div>
      </div>
    </OverlayPortal>
  );
}
