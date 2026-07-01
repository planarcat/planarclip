import { Clipboard, LayoutGrid, LayoutList, Trash2 } from "lucide-react";

import { useRelativeTicker } from "../../hooks/useRelativeTicker";

import type { AppConnectionStatus, ClipEntry, ViewMode } from "../../types";

import { relativeTime } from "../../utils/time";

import { ClipTypeIcon } from "../common/ClipTypeIcon";

import { CopyButton } from "../common/CopyButton";

import { FileClipPreview } from "../common/FileClipPreview";



function ClipPreview({ clip }: { clip: ClipEntry }) {

  if (clip.type === "file") {

    return <FileClipPreview clip={clip} variant="list" />;

  }



  if (clip.type === "image" && clip.imagePreviewUrl) {

    return (

      <div className="overflow-hidden rounded-lg border border-border bg-secondary/30">

        <img

          src={clip.imagePreviewUrl}

          alt={clip.content}

          className="max-h-56 w-full object-contain"

          loading="lazy"

        />

      </div>

    );

  }



  return (

    <p className="line-clamp-3 whitespace-pre-wrap break-all text-sm leading-relaxed text-foreground/90">

      {clip.content}

    </p>

  );

}



function ClipPreviewGrid({ clip }: { clip: ClipEntry }) {

  if (clip.type === "file") {

    return <FileClipPreview clip={clip} variant="grid" />;

  }



  if (clip.type === "image" && clip.imagePreviewUrl) {

    return (

      <div className="flex flex-1 overflow-hidden rounded-lg border border-border bg-secondary/30">

        <img

          src={clip.imagePreviewUrl}

          alt={clip.content}

          className="max-h-48 w-full object-contain"

          loading="lazy"

        />

      </div>

    );

  }



  return (

    <p className="line-clamp-4 flex-1 whitespace-pre-wrap break-all text-sm leading-relaxed text-foreground/90">

      {clip.content}

    </p>

  );

}



type ClipboardPageProps = {

  clips: ClipEntry[];

  viewMode: ViewMode;

  setViewMode: (mode: ViewMode) => void;

  status: AppConnectionStatus;

  statusMessage: string;

  isClearingHistory: boolean;

  onClearHistory: () => void;

};



export function ClipboardPage({

  clips,

  viewMode,

  setViewMode,

  status,

  isClearingHistory,

  onClearHistory,

}: ClipboardPageProps) {

  useRelativeTicker();



  return (

    <div className="flex min-w-0 flex-1 flex-col overflow-y-auto">

      <div className="flex shrink-0 items-center justify-between border-b border-border px-4 pb-3 pt-5 md:px-6">

        <div>

          <h1 className="text-base font-semibold text-primary">剪贴板历史</h1>

          <p className="mt-0.5 text-[13px] font-medium text-muted-foreground">

            最近 {clips.length} 条同步摘要

          </p>

        </div>

        <div className="flex items-center gap-2">

          {clips.length > 0 ? (

            <button

              onClick={onClearHistory}

              disabled={isClearingHistory}

              className="rounded-md p-1.5 text-secondary-foreground transition-colors hover:bg-secondary hover:text-destructive disabled:cursor-not-allowed disabled:opacity-50"

              title={isClearingHistory ? "正在清空" : "清空剪贴板历史"}

              type="button"

            >

              <Trash2 size={14} />

            </button>

          ) : null}

          <div className="flex items-center rounded-md bg-secondary p-0.5">

            <button

              onClick={() => setViewMode("list")}

              className={`rounded p-1.5 transition-colors ${viewMode === "list" ? "bg-card text-primary shadow-sm" : "text-secondary-foreground hover:text-primary"}`}

              title="列表视图"

              type="button"

            >

              <LayoutList size={14} />

            </button>

            <button

              onClick={() => setViewMode("grid")}

              className={`rounded p-1.5 transition-colors ${viewMode === "grid" ? "bg-card text-primary shadow-sm" : "text-secondary-foreground hover:text-primary"}`}

              title="网格视图"

              type="button"

            >

              <LayoutGrid size={14} />

            </button>

          </div>

        </div>

      </div>



      {clips.length === 0 ? (

        <div className="flex flex-1 items-center justify-center px-6 py-10">

          <div className="max-w-sm rounded-2xl border border-dashed border-border bg-card/80 px-6 py-8 text-center">

            <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary/10 text-primary">

              <Clipboard size={24} />

            </div>

            <p className="text-sm font-medium text-primary">

              {status === "online" ? "等待新的同步内容" : "连接建立后，这里会显示最近的同步摘要"}

            </p>

          </div>

        </div>

      ) : viewMode === "list" ? (

        <div className="flex-1">

          {clips.map((clip) => {

            const sourceLine = clip.direction === "received" ? `来自 ${clip.sourceLabel}` : `从 ${clip.sourceLabel} 发出`;

            const showTypeIcon = clip.type !== "file";



            return (

              <div key={clip.id} className="group border-b border-border px-4 py-4 transition-colors last:border-0 hover:bg-secondary/40 md:px-6">

                <div className="flex items-start gap-3">

                  {showTypeIcon ? <ClipTypeIcon type={clip.type} /> : null}

                  <div className="min-w-0 flex-1">

                    <div className="mb-1.5 flex items-center gap-2">

                      <span className="text-[13px] font-medium text-primary">{sourceLine}</span>

                      <span className="ml-auto shrink-0 text-[13px] font-medium text-muted-foreground">{relativeTime(clip.timestamp)}</span>

                      {clip.type !== "file" ? (

                        <span className="shrink-0 font-mono text-[13px] font-medium text-secondary-foreground">{clip.size}</span>

                      ) : null}

                      <div className="opacity-0 transition-opacity group-hover:opacity-100">

                        {clip.type === "text" ? <CopyButton text={clip.content} /> : null}

                      </div>

                    </div>

                    <ClipPreview clip={clip} />

                  </div>

                </div>

              </div>

            );

          })}

        </div>

      ) : (

        <div className="grid content-start gap-3 p-4 md:p-5 xl:grid-cols-3 2xl:grid-cols-4">

          {clips.map((clip) => {

            const sourceLine = clip.direction === "received" ? `来自 ${clip.sourceLabel}` : `从 ${clip.sourceLabel} 发出`;

            const showTypeIcon = clip.type !== "file";



            return (

              <div key={clip.id} className="group flex flex-col overflow-hidden rounded-xl border border-border bg-card transition-colors hover:border-primary/30">

                <div className="flex flex-1 flex-col gap-2 p-3">

                  <div className="flex items-center gap-2">

                    {showTypeIcon ? <ClipTypeIcon type={clip.type} /> : null}

                    <span className="truncate text-[13px] font-medium text-primary">{sourceLine}</span>

                    <div className="ml-auto opacity-0 transition-opacity group-hover:opacity-100">

                      {clip.type === "text" ? <CopyButton text={clip.content} /> : null}

                    </div>

                  </div>

                  <ClipPreviewGrid clip={clip} />

                  <div className="mt-auto flex items-center justify-between border-t border-border pt-2">

                    <span className="text-[13px] font-medium text-muted-foreground">{relativeTime(clip.timestamp)}</span>

                    <span className="font-mono text-[13px] font-medium text-secondary-foreground">{clip.size}</span>

                  </div>

                </div>

              </div>

            );

          })}

        </div>

      )}

    </div>

  );

}


