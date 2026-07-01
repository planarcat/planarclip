import { File, FileText, Image } from "lucide-react";
import type { ClipType } from "../../types";

type ClipTypeIconProps = {
  type: ClipType;
};

export function ClipTypeIcon({ type }: ClipTypeIconProps) {
  if (type === "image") {
    return (
      <span className="shrink-0 rounded bg-violet-400/10 p-1.5 text-violet-400">
        <Image size={14} />
      </span>
    );
  }

  if (type === "file") {
    return (
      <span className="shrink-0 rounded bg-amber-400/10 p-1.5 text-amber-400">
        <File size={14} />
      </span>
    );
  }

  return (
    <span className="shrink-0 rounded bg-primary/10 p-1.5 text-primary">
      <FileText size={14} />
    </span>
  );
}
