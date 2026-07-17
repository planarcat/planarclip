import { ChevronDown, ChevronUp, File } from "lucide-react";
import { useMemo, useState } from "react";
import type { ClipEntry } from "../../types";
import { fileExtension, useHistoryMediaUrl, useTypeIconUrl } from "../../utils/clipboard";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";
import { ClipTypeIcon } from "./ClipTypeIcon";

type FileClipPreviewProps = {
  clip: ClipEntry;
  variant?: "list" | "grid";
};

function FilePreviewImage({
  url,
  alt,
  compact,
  isIcon,
  onError,
}: {
  url: string;
  alt: string;
  compact?: boolean;
  isIcon?: boolean;
  onError?: () => void;
}) {
  if (isIcon) {
    return (
      <div
        className={`flex shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border bg-secondary/30 ${
          compact ? "h-16 w-16" : "h-20 w-20"
        }`}
      >
        <img
          src={url}
          alt={alt}
          className="h-10 w-10 object-contain"
          loading="lazy"
          onError={onError}
        />
      </div>
    );
  }

  return (
    <div
      className={`overflow-hidden rounded-lg border border-border bg-secondary/30 ${
        compact ? "max-h-40" : "max-h-56"
      }`}
    >
      <img
        src={url}
        alt={alt}
        className="h-full w-full object-contain"
        loading="lazy"
        onError={onError}
      />
    </div>
  );
}

export function FileClipPreview({ clip, variant = "list" }: FileClipPreviewProps) {
  const [expanded, setExpanded] = useState(false);
  const mediaUrl = useHistoryMediaUrl(clip.mediaRef);
  const iconExt =
    clip.previewKind === "icon"
      ? fileExtension(clip.fileNames?.[0] ?? clip.content)
      : undefined;
  const iconUrl = useTypeIconUrl(iconExt);

  const fileCount = clip.fileCount ?? clip.fileNames?.length ?? 1;
  const isMultiFile = fileCount > 1;
  const fileNames = clip.fileNames ?? (isMultiFile ? [] : [clip.content]);
  const canExpand = isMultiFile && fileNames.length > 0;

  const preview = useMemo(() => {
    if (clip.previewKind === "icon") {
      if (iconUrl) {
        return (
          <FilePreviewImage
            url={iconUrl}
            alt={clip.content}
            compact={variant === "grid"}
          />
        );
      }
      return <ClipTypeIcon type="file" />;
    }
    if (mediaUrl) {
      return (
        <FilePreviewImage
          url={mediaUrl}
          alt={clip.content}
          compact={variant === "grid"}
        />
      );
    }

    if (!isMultiFile) {
      return (
        <div
          className={`flex shrink-0 items-center justify-center rounded-lg border border-border bg-secondary/30 text-amber-400 ${
            variant === "grid" ? "h-16 w-16" : "h-20 w-20"
          }`}
        >
          <File size={variant === "grid" ? 24 : 28} />
        </div>
      );
    }

    return null;
  }, [clip.content, clip.previewKind, isMultiFile, mediaUrl, iconUrl, variant]);

  if (isMultiFile) {
    return (
      <div className="space-y-2">
        <div className="flex items-center gap-3">
          {preview ?? <ClipTypeIcon type="file" />}
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-medium text-foreground" title={clip.content}>
              {clip.content}
            </p>
            <p className="mt-0.5 text-[12px] font-medium text-muted-foreground">
              {clip.size} · {fileCount} 个文件
            </p>
          </div>
        </div>

        {canExpand ? (
          <>
            <button
              type="button"
              onClick={() => setExpanded((value) => !value)}
              className={`inline-flex items-center gap-1 rounded px-1 py-0.5 text-[12px] font-medium text-primary ${SURFACE_REVEAL_BG} hover:bg-primary/10 hover:text-primary/80`}
            >
              {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
              {expanded ? "收起列表" : "展开文件列表"}
            </button>

            {expanded ? (
              <ul className="max-h-48 space-y-1 overflow-y-auto rounded-lg border border-border bg-secondary/20 px-3 py-2">
                {fileNames.map((name, index) => (
                  <li
                    key={`${name}-${index}`}
                    className="truncate text-[12px] leading-5 text-foreground/90"
                    title={name}
                  >
                    · {name}
                  </li>
                ))}
              </ul>
            ) : null}
          </>
        ) : null}
      </div>
    );
  }

  return (
    <div className={`flex gap-3 ${variant === "grid" ? "flex-col" : "items-center"}`}>
      {preview}
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-foreground" title={clip.content}>
          {clip.content}
        </p>
        <p className="mt-0.5 text-[12px] font-medium text-muted-foreground">{clip.size}</p>
      </div>
    </div>
  );
}
