import type { ReactNode } from "react";
import { createPortal } from "react-dom";

type OverlayPortalProps = {
  children: ReactNode;
};

export function OverlayPortal({ children }: OverlayPortalProps) {
  return createPortal(children, document.body);
}
