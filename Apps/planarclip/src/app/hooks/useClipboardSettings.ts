import { useCallback, useEffect, useState } from "react";
import { DEFAULT_CLIPBOARD_HISTORY_LIMIT } from "../constants/clipboard";
import type { ClipEntry, CommandExecutor, ClipboardSettingsPayload } from "../types";
import { normalizeUserMessage } from "../utils/message";

type UseClipboardSettingsOptions = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
  setLastMessage: (message: string) => void;
  setClips: (clips: ClipEntry[]) => void;
};

export function useClipboardSettings({
  tauriAvailable,
  callCommand,
  setLastMessage,
  setClips,
}: UseClipboardSettingsOptions) {
  const [historyLimit, setHistoryLimit] = useState(DEFAULT_CLIPBOARD_HISTORY_LIMIT);
  const [isSaving, setIsSaving] = useState(false);
  const [isClearing, setIsClearing] = useState(false);
  const [loaded, setLoaded] = useState(!tauriAvailable);

  useEffect(() => {
    if (!tauriAvailable) {
      return;
    }

    let disposed = false;

    const load = async () => {
      try {
        const settings = await callCommand<ClipboardSettingsPayload>("get_clipboard_settings");
        if (disposed) {
          return;
        }
        setHistoryLimit(settings.history_limit);
      } catch {
        if (!disposed) {
          setHistoryLimit(DEFAULT_CLIPBOARD_HISTORY_LIMIT);
        }
      } finally {
        if (!disposed) {
          setLoaded(true);
        }
      }
    };

    void load();

    return () => {
      disposed = true;
    };
  }, [callCommand, tauriAvailable]);

  const handleHistoryLimitChange = useCallback(
    async (limit: number) => {
      if (!tauriAvailable) {
        setHistoryLimit(limit);
        setLastMessage("当前是浏览器预览模式，剪贴板设置仅用于界面预览。");
        return;
      }

      setIsSaving(true);
      try {
        const saved = await callCommand<ClipboardSettingsPayload>("save_clipboard_settings", {
          historyLimit: limit,
        });
        setHistoryLimit(saved.history_limit);
        setLastMessage(`已更新展示上限为 ${saved.history_limit} 条。`);
      } catch (error) {
        setLastMessage(normalizeUserMessage(error, "这次没有保存成功，请稍后再试。"));
      } finally {
        setIsSaving(false);
      }
    },
    [callCommand, setLastMessage, tauriAvailable],
  );

  const handleClearHistory = useCallback(async () => {
    if (!tauriAvailable) {
      setClips([]);
      setLastMessage("当前是浏览器预览模式，清空操作仅影响界面预览。");
      return;
    }

    setIsClearing(true);
    try {
      await callCommand("clear_clipboard_history");
      setClips([]);
      setLastMessage("已清空剪贴板历史。");
    } catch (error) {
      setLastMessage(normalizeUserMessage(error, "清空失败，请稍后再试。"));
    } finally {
      setIsClearing(false);
    }
  }, [callCommand, setClips, setLastMessage, tauriAvailable]);

  return {
    historyLimit,
    isSavingClipboardSettings: isSaving,
    clipboardSettingsLoaded: loaded,
    isClearingClipboardHistory: isClearing,
    handleHistoryLimitChange,
    handleClearHistory,
  };
}
