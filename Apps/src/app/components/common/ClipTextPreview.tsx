import { X } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import type { ClipEntry } from "../../types";
import { useHistoryMediaUrl } from "../../utils/clipboard";

import { CLIP_LIST_PREVIEW_SURFACE } from "../../constants/clipPreviewSurface";
import { IconButton } from "../ui/IconButton";
import { ModalShell } from "../ui/ModalShell";
import { ScrollArea } from "../ui/ScrollArea";
import { ClipHistoryActions } from "./ClipHistoryActions";
import { ClipImageZoomView } from "./ClipImageZoomView";

type ClipDetailModalProps = {
  open: boolean;
  clip: ClipEntry;
  onClose: () => void;
  onActionMessage?: (message: string) => void;
};

function ClipDetailModal({
  open,
  clip,
  onClose,
  onActionMessage,
}: ClipDetailModalProps) {
  const { content } = clip;
  const imageUrl = useHistoryMediaUrl(clip.mediaRef);
  const isImage = clip.type === "image" && (Boolean(imageUrl) || Boolean(clip.mediaRef));
  const title = isImage ? "图片预览" : "完整内容";

  useEffect(() => {
    if (!open) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  const panelClassName = isImage
    ? "flex max-h-[min(85vh,720px)] max-w-[min(90vw,960px)] flex-col"
    : "flex max-h-[min(80vh,640px)] max-w-2xl flex-col";

  return (
    <ModalShell
      open={open}
      onBackdropClick={onClose}
      zIndexClassName="z-[55]"
      labelledBy="clip-detail-modal-title"
      panelClassName={panelClassName}
    >
      <div className="flex shrink-0 items-center justify-between border-b border-border px-5 py-4">
        <p id="clip-detail-modal-title" className="text-sm font-semibold text-foreground">
          {title}
        </p>
        <div className="flex items-center gap-0.5">
          <ClipHistoryActions clip={clip} onActionMessage={onActionMessage} />
          <IconButton onClick={onClose} title="关闭" aria-label="关闭">
            <X size={15} />
          </IconButton>
        </div>
      </div>
      {isImage ? (
        imageUrl ? (
          <ClipImageZoomView src={imageUrl} alt={content} resetKey={`${clip.id}-${open}`} />
        ) : (
          <div className="flex min-h-[200px] flex-1 items-center justify-center">
            <span className="text-sm text-muted-foreground">图片加载中…</span>
          </div>
        )
      ) : (
        <ScrollArea className="min-h-0 flex-1 overflow-y-auto p-5">
          <p className="whitespace-pre-wrap break-all text-sm leading-relaxed text-foreground/90">{content}</p>
        </ScrollArea>
      )}
    </ModalShell>
  );
}

type ClipTextPreviewProps = {
  clip: ClipEntry;
  lineClamp: 3 | 4;
  className?: string;
  onActionMessage?: (message: string) => void;
};

export function ClipTextPreview({
  clip,
  lineClamp,
  className = "",
  onActionMessage,
}: ClipTextPreviewProps) {
  const { content } = clip;
  const textRef = useRef<HTMLDivElement>(null);
  const [truncated, setTruncated] = useState(false);
  const [open, setOpen] = useState(false);

  const checkTruncation = useCallback(() => {
    const el = textRef.current;
    if (!el) {
      return;
    }
    setTruncated(el.scrollHeight > el.clientHeight + 1);
  }, []);

  useLayoutEffect(() => {
    checkTruncation();
    window.addEventListener("resize", checkTruncation);
    return () => window.removeEventListener("resize", checkTruncation);
  }, [content, lineClamp, checkTruncation]);

  const clampClass = lineClamp === 3 ? "line-clamp-3" : "line-clamp-4";
  const textBodyClass = `whitespace-pre-wrap break-all text-sm leading-relaxed text-foreground/90 ${className}`;
  const openDetail = () => setOpen(true);

  return (
    <>
      <div
        ref={textRef}
        role={truncated ? "button" : undefined}
        tabIndex={truncated ? 0 : undefined}
        title={truncated ? "查看完整内容" : undefined}
        aria-label={truncated ? "查看完整内容" : undefined}
        onClick={truncated ? openDetail : undefined}
        onKeyDown={
          truncated
            ? (event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  openDetail();
                }
              }
            : undefined
        }
        className={`${clampClass} ${textBodyClass} ${truncated ? "cursor-pointer rounded hover:text-primary" : ""}`}
      >
        {content}
      </div>
      <ClipDetailModal
        open={open}
        clip={clip}
        onClose={() => setOpen(false)}
        onActionMessage={onActionMessage}
      />
    </>
  );
}

type ClipImagePreviewProps = {
  clip: ClipEntry;
  variant?: "list" | "grid";
  onActionMessage?: (message: string) => void;
};

export function ClipImagePreview({
  clip,
  variant = "list",
  onActionMessage,
}: ClipImagePreviewProps) {
  const [open, setOpen] = useState(false);
  const url = useHistoryMediaUrl(clip.mediaRef);

  if (!url) {
    if (!clip.mediaRef) {
      return null;
    }
    const placeholderClass = variant === "grid" ? "h-48" : "h-56";
    return (
      <div
        className={`flex w-full items-center justify-center rounded-lg border border-border bg-secondary/30 ${placeholderClass}`}
        aria-label="图片加载中"
      >
        <span className="text-xs text-muted-foreground">加载中…</span>
      </div>
    );
  }

  const maxThumbClass = variant === "grid" ? "max-h-48" : "max-h-56";
  const wrapperClass =
    variant === "grid" ? "flex flex-1 overflow-hidden rounded-lg" : "overflow-hidden rounded-lg";

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        title="查看大图"
        aria-label="查看大图"
        className={`group block w-full ${CLIP_LIST_PREVIEW_SURFACE} ${wrapperClass}`}
      >
        <img
          src={url}
          alt={clip.content}
          className={`${maxThumbClass} w-full object-contain transition-opacity group-hover:opacity-95`}
          loading="lazy"
        />
      </button>
      <ClipDetailModal
        open={open}
        clip={clip}
        onClose={() => setOpen(false)}
        onActionMessage={onActionMessage}
      />
    </>
  );
}
