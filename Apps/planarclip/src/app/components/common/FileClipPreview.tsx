import { ChevronDown, ChevronUp, File } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { ClipEntry } from "../../types";
import { resolveHistoryThumbnailUrl } from "../../utils/clipboard";
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
  const [thumbnailUrl, setThumbnailUrl] = useState<string | undefined>();

  const fileCount = clip.fileCount ?? clip.fileNames?.length ?? 1;
  const isMultiFile = fileCount > 1;
  const fileNames = clip.fileNames ?? (isMultiFile ? [] : [clip.content]);
  const canExpand = isMultiFile && fileNames.length > 0;

  useEffect(() => {
    if (!clip.thumbnailRef) {
      setThumbnailUrl(undefined);
      return;
    }

    let cancelled = false;
    resolveHistoryThumbnailUrl(clip.thumbnailRef).then((url) => {
      if (!cancelled) {
        setThumbnailUrl(url);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [clip.thumbnailRef]);

  const preview = useMemo(() => {
    if (thumbnailUrl) {
      return (
        <FilePreviewImage
          url={thumbnailUrl}
          alt={clip.content}
          compact={variant === "grid"}
          isIcon={clip.previewKind === "icon"}
          onError={() => setThumbnailUrl(undefined)}
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
  }, [clip.content, clip.previewKind, isMultiFile, thumbnailUrl, variant]);

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
              className="inline-flex items-center gap-1 text-[12px] font-medium text-primary transition-colors hover:text-primary/80"
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
