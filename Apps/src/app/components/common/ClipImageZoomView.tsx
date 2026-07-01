import { Maximize2, ZoomIn, ZoomOut } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { IconButton } from "../ui/IconButton";

const MIN_SCALE = 0.25;
const MAX_SCALE = 4;

function clampScale(value: number) {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, value));
}

type ClipImageZoomViewProps = {
  src: string;
  alt: string;
  resetKey: string;
};

export function ClipImageZoomView({ src, alt, resetKey }: ClipImageZoomViewProps) {
  const [scale, setScale] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const viewportRef = useRef<HTMLDivElement>(null);
  const panRef = useRef(pan);
  panRef.current = pan;

  const dragRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    panX: number;
    panY: number;
  } | null>(null);

  useEffect(() => {
    setScale(1);
    setPan({ x: 0, y: 0 });
  }, [resetKey, src]);

  const resetView = useCallback(() => {
    setScale(1);
    setPan({ x: 0, y: 0 });
  }, []);

  const zoomIn = useCallback(() => {
    setScale((current) => clampScale(current * 1.2));
  }, []);

  const zoomOut = useCallback(() => {
    setScale((current) => clampScale(current / 1.2));
  }, []);

  useEffect(() => {
    const el = viewportRef.current;
    if (!el) {
      return;
    }

    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const delta = event.deltaY > 0 ? 0.9 : 1.1;
      setScale((current) => clampScale(current * delta));
    };

    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  useEffect(() => {
    const onPointerMove = (event: PointerEvent) => {
      const drag = dragRef.current;
      if (!drag || drag.pointerId !== event.pointerId) {
        return;
      }
      setPan({
        x: drag.panX + (event.clientX - drag.startX),
        y: drag.panY + (event.clientY - drag.startY),
      });
    };

    const endDrag = (event: PointerEvent) => {
      const drag = dragRef.current;
      if (!drag || drag.pointerId !== event.pointerId) {
        return;
      }
      dragRef.current = null;
    };

    document.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerup", endDrag);
    document.addEventListener("pointercancel", endDrag);

    return () => {
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerup", endDrag);
      document.removeEventListener("pointercancel", endDrag);
    };
  }, []);

  const onPointerDown = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    dragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      panX: panRef.current.x,
      panY: panRef.current.y,
    };
  }, []);

  const onDoubleClick = useCallback(() => {
    resetView();
  }, [resetView]);

  const canPan = scale > 1.01 || Math.abs(pan.x) > 0.5 || Math.abs(pan.y) > 0.5;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex shrink-0 items-center justify-end gap-0.5 border-b border-border px-3 py-2">
        <IconButton size="sm" onClick={zoomOut} title="缩小" aria-label="缩小">
          <ZoomOut size={14} />
        </IconButton>
        <span className="min-w-[3rem] text-center text-xs font-medium tabular-nums text-muted-foreground">
          {Math.round(scale * 100)}%
        </span>
        <IconButton size="sm" onClick={zoomIn} title="放大" aria-label="放大">
          <ZoomIn size={14} />
        </IconButton>
        <IconButton size="sm" onClick={resetView} title="重置缩放" aria-label="重置缩放">
          <Maximize2 size={14} />
        </IconButton>
      </div>
      <div
        ref={viewportRef}
        className={`min-h-0 flex-1 touch-none overflow-hidden bg-secondary/20 p-4 select-none ${
          canPan ? "cursor-grab active:cursor-grabbing" : ""
        }`}
        onPointerDown={onPointerDown}
        onDoubleClick={onDoubleClick}
        role="presentation"
        title="滚轮缩放，拖拽移动，双击重置"
      >
        <div
          className="flex h-full w-full items-start justify-center"
          style={{
            transform: `translate(${pan.x}px, ${pan.y}px)`,
          }}
        >
          <img
            src={src}
            alt={alt}
            draggable={false}
            className="pointer-events-none max-h-[min(70vh,640px)] max-w-full origin-center object-contain"
            style={{
              transform: `scale(${scale})`,
            }}
          />
        </div>
      </div>
    </div>
  );
}
