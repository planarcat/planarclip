import { FolderOpen } from "lucide-react";
import { useEffect, useState } from "react";
import type { CommandExecutor } from "../../types";
import { logger } from "../../utils/logger";
import { SettingToggleControl } from "./SettingToggle";

type DiagnosticsPanelProps = {
  tauriAvailable: boolean;
  callCommand: CommandExecutor;
};

/** Self-contained diagnostics section: verbose-log toggle + open log directory. */
export function DiagnosticsPanel({ tauriAvailable, callCommand }: DiagnosticsPanelProps) {
  const [verboseLog, setVerboseLog] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isOpening, setIsOpening] = useState(false);

  useEffect(() => {
    if (!tauriAvailable) {
      setLoaded(true);
      return;
    }
    let cancelled = false;
    void callCommand<boolean>("get_diagnostic_settings")
      .then((value) => {
        if (!cancelled) {
          setVerboseLog(value);
          setLoaded(true);
        }
      })
      .catch((error) => {
        if (!cancelled) {
          logger.warn("diagnostics", `failed to load diagnostic settings: ${String(error)}`);
          setLoaded(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [tauriAvailable, callCommand]);

  const handleVerboseChange = (next: boolean) => {
    setIsSaving(true);
    void callCommand<boolean>("save_diagnostic_settings", { verboseLog: next })
      .then((value) => {
        setVerboseLog(value);
        logger.info("diagnostics", `verbose logging ${value ? "enabled" : "disabled"}`);
      })
      .catch((error) => {
        logger.warn("diagnostics", `failed to save diagnostic settings: ${String(error)}`);
      })
      .finally(() => setIsSaving(false));
  };

  const handleOpenLogDir = () => {
    setIsOpening(true);
    void callCommand<string>("open_log_dir")
      .catch((error) => {
        logger.warn("diagnostics", `failed to open log dir: ${String(error)}`);
      })
      .finally(() => setIsOpening(false));
  };

  return (
    <>
      <div className="flex items-center justify-between gap-4 border-b border-border py-3.5">
        <div>
          <p className="text-sm font-medium text-primary">详细日志</p>
          <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
            开启后立即将日志级别提升为 debug，便于排查问题；关闭后恢复默认。日志仅记录本机行为，不会上传。
          </p>
        </div>
        <SettingToggleControl
          checked={verboseLog}
          disabled={!tauriAvailable || !loaded || isSaving}
          label="详细日志"
          onChange={handleVerboseChange}
        />
      </div>
      <div className="flex items-center justify-between gap-4 py-3.5">
        <div>
          <p className="text-sm font-medium text-primary">日志目录</p>
          <p className="mt-0.5 text-[13px] font-medium leading-6 text-muted-foreground">
            打开存放运行日志的文件夹，日志按天滚动并保留 7 天。
          </p>
        </div>
        <button
          type="button"
          onClick={handleOpenLogDir}
          disabled={!tauriAvailable || isOpening}
          className="flex shrink-0 items-center gap-2 rounded-lg border border-border bg-transparent px-3 py-2 text-sm font-medium text-secondary-foreground hover:border-primary/40 hover:bg-secondary/40 hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
          title="打开日志目录"
        >
          <FolderOpen size={14} />
          打开
        </button>
      </div>
    </>
  );
}
