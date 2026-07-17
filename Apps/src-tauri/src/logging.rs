//! Centralized production logging.
//!
//! - Daily-rotated file appender under the app data root (`logs/`, sibling of `staging/`).
//! - Stderr mirror retained for console-attached debugging (no-op on the GUI subsystem).
//! - Non-blocking writes so hot paths (clipboard poll, network) never block on IO.
//! - Panic hook captures panics into the log (GUI subsystem has no console).
//! - Startup cleanup of rotated logs older than `LOG_RETENTION_DAYS`.
//! - Runtime level reload backing the in-app diagnostics toggle (`set_verbose`).
//! - Frontend log bridge (`frontend_log`) so renderer logs land in the same file/timeline.
//! - Redaction helpers to keep clipboard plaintext / keys / pairing codes out of logs.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;

use crate::storage::json;

const DEFAULT_FILTER_RELEASE: &str = "info";
const DEFAULT_FILTER_DEV: &str = "info,mdns_sd=off";
const VERBOSE_FILTER: &str = "debug,mdns_sd=off";
const LOG_RETENTION_DAYS: u64 = 7;
const LOG_DIR_NAME: &str = "logs";

/// Target tag used for log lines originating in the frontend renderer.
pub const FRONTEND_TARGET: &str = "frontend";

type ReloadHandle = reload::Handle<EnvFilter, tracing_subscriber::Registry>;

static LEVEL_RELOAD: OnceLock<ReloadHandle> = OnceLock::new();

/// Initialize the global tracing subscriber.
///
/// Returns a `WorkerGuard` whose lifetime flushes the non-blocking file writer;
/// the caller must keep it alive for the whole process (store it in `run`).
pub fn init() -> WorkerGuard {
    let dir = log_dir();
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!("failed to create log dir {}: {error}", dir.display());
    }

    let file_appender = tracing_appender::rolling::daily(&dir, log_file_prefix());
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let (filter_layer, reload_handle) = reload::Layer::new(build_filter(false));
    let _ = LEVEL_RELOAD.set(reload_handle);

    let stderr_layer = tracing_subscriber::fmt::layer().with_target(true);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_target(true)
        .with_ansi(false);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(stderr_layer)
        .with(file_layer)
        .init();

    install_panic_hook();
    cleanup_old_logs(&dir);

    tracing::info!(target: "logging", log_dir = %dir.display(), "logging initialized");
    guard
}

/// Resolve the log directory (sibling of `staging/` and `history_thumbs/`).
pub fn log_dir() -> PathBuf {
    json::config_path()
        .parent()
        .map(|parent| parent.join(LOG_DIR_NAME))
        .unwrap_or_else(|| PathBuf::from(LOG_DIR_NAME))
}

fn build_filter(verbose: bool) -> EnvFilter {
    // RUST_LOG always wins; the verbose toggle only changes the fallback default.
    if verbose {
        return EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(VERBOSE_FILTER));
    }
    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(if cfg!(debug_assertions) {
            DEFAULT_FILTER_DEV
        } else {
            DEFAULT_FILTER_RELEASE
        })
    })
}

/// Raise/lower the log level at runtime for the in-app diagnostics toggle.
pub fn set_verbose(verbose: bool) {
    if let Some(handle) = LEVEL_RELOAD.get() {
        if let Err(error) = handle.modify(|filter| *filter = build_filter(verbose)) {
            tracing::warn!(target: "logging", "failed to reload log level: {error}");
        }
    }
}

fn log_file_prefix() -> &'static str {
    if cfg!(debug_assertions) {
        "planarclip-dev.log"
    } else {
        "planarclip.log"
    }
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".to_string()
        };
        tracing::error!(target: "panic", location = %location, "panic: {payload}");
        previous(info);
    }));
}

fn cleanup_old_logs(dir: &Path) {
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(60 * 60 * 24 * LOG_RETENTION_DAYS);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Only touch our own rotated log files; never delete unrelated entries.
        let is_ours = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("planarclip"))
            .unwrap_or(false);
        if !is_ours {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }
}

/// Frontend -> backend log bridge. Renderer logs land in the same file/timeline.
#[tauri::command]
pub fn frontend_log(level: String, target: String, message: String) {
    // tracing's `target:` requires a static string, so embed the frontend
    // sub-target (e.g. "pairing") into the message line instead.
    let line = if target.is_empty() {
        message
    } else {
        format!("[{target}] {message}")
    };
    match level.as_str() {
        "error" => tracing::error!(target: FRONTEND_TARGET, "{line}"),
        "warn" => tracing::warn!(target: FRONTEND_TARGET, "{line}"),
        "info" => tracing::info!(target: FRONTEND_TARGET, "{line}"),
        "debug" => tracing::debug!(target: FRONTEND_TARGET, "{line}"),
        _ => tracing::trace!(target: FRONTEND_TARGET, "{line}"),
    }
}

/// Open the log directory in the platform file manager.
#[tauri::command]
pub fn open_log_dir() -> Result<String, String> {
    let dir = log_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create log dir failed: {e}"))?;
    open_path(&dir).map_err(|e| format!("open log dir failed: {e}"))?;
    Ok(dir.to_string_lossy().to_string())
}

#[cfg(target_os = "windows")]
fn open_path(path: &Path) -> std::io::Result<()> {
    std::process::Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "macos")]
fn open_path(path: &Path) -> std::io::Result<()> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn open_path(path: &Path) -> std::io::Result<()> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
}

// ---- redaction ----

/// Summarize clipboard content for logging without exposing plaintext.
/// Records only kind, byte size, and a short blake3 hash prefix.
pub fn redact_content(kind: &str, size: usize, hash: &[u8]) -> String {
    let prefix_len = hash.len().min(4);
    let prefix = if prefix_len > 0 {
        hex::encode(&hash[..prefix_len])
    } else {
        String::from("-")
    };
    format!("kind={kind} size={size} hash={prefix}..")
}

/// Shorten a peer id to an unambiguous prefix for logging.
pub fn redact_peer(peer_id: &str) -> String {
    let len = peer_id.chars().count();
    if len <= 8 {
        peer_id.to_string()
    } else {
        let prefix: String = peer_id.chars().take(8).collect();
        format!("{prefix}…")
    }
}
