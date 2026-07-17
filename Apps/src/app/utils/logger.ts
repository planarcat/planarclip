import { invoke, isTauri } from "@tauri-apps/api/core";

type LogLevel = "error" | "warn" | "info" | "debug";

/**
 * Emit a log line to both the devtools console and the Rust tracing pipeline.
 *
 * The renderer forwards `frontend_log` to the backend so that frontend-originated
 * events land in the same rotated log file as the Rust backend, on a shared
 * timeline. Backend invocation failures are swallowed: logging must never throw
 * into the calling code.
 */
function emit(level: LogLevel, target: string, message: string, ...rest: unknown[]): void {
  const consoleArgs =
    rest.length > 0 ? [`[${target}]`, message, ...rest] : [`[${target}]`, message];
  switch (level) {
    case "error":
      console.error(...consoleArgs);
      break;
    case "warn":
      console.warn(...consoleArgs);
      break;
    case "info":
      console.info(...consoleArgs);
      break;
    default:
      console.debug(...consoleArgs);
      break;
  }

  if (isTauri()) {
    void invoke("frontend_log", { level, target, message }).catch(() => {
      // Logging must never propagate errors into the caller.
    });
  }
}

/** Structured frontend logger. `target` groups lines by feature (e.g. "pairing"). */
export const logger = {
  error: (target: string, message: string, ...rest: unknown[]) =>
    emit("error", target, message, ...rest),
  warn: (target: string, message: string, ...rest: unknown[]) =>
    emit("warn", target, message, ...rest),
  info: (target: string, message: string, ...rest: unknown[]) =>
    emit("info", target, message, ...rest),
  debug: (target: string, message: string, ...rest: unknown[]) =>
    emit("debug", target, message, ...rest),
};

/** Capture uncaught errors and unhandled promise rejections into the log. */
export function installGlobalErrorCapture(): void {
  window.addEventListener("error", (event) => {
    const loc = event.filename
      ? ` @ ${event.filename}:${event.lineno}:${event.colno}`
      : "";
    emit("error", "window", `uncaught error: ${event.message}${loc}`);
  });
  window.addEventListener("unhandledrejection", (event) => {
    const reason =
      event.reason instanceof Error
        ? `${event.reason.name}: ${event.reason.message}`
        : String(event.reason);
    emit("error", "window", `unhandled promise rejection: ${reason}`);
  });
}
