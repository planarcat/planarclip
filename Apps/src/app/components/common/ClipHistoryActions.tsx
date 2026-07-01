import { invoke, isTauri } from "@tauri-apps/api/core";

import { Copy, Send, ShieldCheck } from "lucide-react";

import { useState } from "react";



import type { ClipEntry } from "../../types";

import { normalizeUserMessage } from "../../utils/message";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";



type ClipHistoryActionsProps = {

  clip: ClipEntry;

  showSendButton?: boolean;

  onActionMessage?: (message: string) => void;

};



const TAURI_AVAILABLE = isTauri();



export function ClipHistoryActions({

  clip,

  showSendButton = false,

  onActionMessage,

}: ClipHistoryActionsProps) {

  const [copied, setCopied] = useState(false);

  const [sending, setSending] = useState(false);



  const isFile = clip.type === "file";



  if (isFile && !showSendButton) {

    return null;

  }



  if (!isFile && !showSendButton) {

    return (

      <div className="flex items-center gap-0.5">

        <button

          onClick={() => {

            if (TAURI_AVAILABLE) {

              void invoke("copy_clipboard_history_entry", { entryId: clip.id })

                .then(() => {

                  setCopied(true);

                  window.setTimeout(() => setCopied(false), 1_600);

                })

                .catch((error) => {

                  onActionMessage?.(normalizeUserMessage(error, "未能写入剪贴板，请稍后再试。"));

                });

              return;

            }



            navigator.clipboard.writeText(clip.content).catch(() => undefined);

            setCopied(true);

            window.setTimeout(() => setCopied(false), 1_600);

          }}

          className={`rounded p-1.5 text-muted-foreground ${SURFACE_REVEAL_BG} hover:bg-primary/10 hover:text-primary`}

          title="复制到剪贴板（不同步）"

          type="button"

        >

          {copied ? <ShieldCheck size={13} /> : <Copy size={13} />}

        </button>

      </div>

    );

  }



  const handleCopy = () => {

    if (isFile) {

      return;

    }

    if (TAURI_AVAILABLE) {

      void invoke("copy_clipboard_history_entry", { entryId: clip.id })

        .then(() => {

          setCopied(true);

          window.setTimeout(() => setCopied(false), 1_600);

        })

        .catch((error) => {

          onActionMessage?.(normalizeUserMessage(error, "未能写入剪贴板，请稍后再试。"));

        });

      return;

    }



    navigator.clipboard.writeText(clip.content).catch(() => undefined);

    setCopied(true);

    window.setTimeout(() => setCopied(false), 1_600);

  };



  const handleSend = () => {

    if (!TAURI_AVAILABLE) {

      onActionMessage?.("当前是浏览器预览模式，同步需在桌面应用中操作。");

      return;

    }



    if (sending) {

      return;

    }



    setSending(true);

    void invoke("send_clipboard_history_entry", { entryId: clip.id })

      .then(() => {

        onActionMessage?.(isFile ? "已开始同步文件。" : "已开始同步。");

      })

      .catch((error) => {

        onActionMessage?.(normalizeUserMessage(error, "未能同步，请确认已连接设备后重试。"));

      })

      .finally(() => {

        setSending(false);

      });

  };



  return (

    <div className="flex items-center gap-0.5">

      {!isFile ? (

        <button

          onClick={handleCopy}

          className={`rounded p-1.5 text-muted-foreground ${SURFACE_REVEAL_BG} hover:bg-primary/10 hover:text-primary`}

          title="复制到剪贴板（不同步）"

          type="button"

        >

          {copied ? <ShieldCheck size={13} /> : <Copy size={13} />}

        </button>

      ) : null}

      <button

        onClick={handleSend}

        disabled={sending}

        className={`rounded p-1.5 text-muted-foreground ${SURFACE_REVEAL_BG} hover:bg-primary/10 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50`}

        title="同步"

        aria-label="同步"

        type="button"

      >

        <Send size={13} />

      </button>

    </div>

  );

}


