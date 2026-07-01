import { Apple, Monitor } from "lucide-react";
import type { OS } from "../../types";

type OsIconProps = {
  os: OS;
  size?: number;
};

export function OsIcon({ os, size = 14 }: OsIconProps) {
  return os === "macos" ? <Apple size={size} className="shrink-0" /> : <Monitor size={size} className="shrink-0" />;
}
