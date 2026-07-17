//! Per-instance data directory resolution.
//!
//! All app data lives under `<data_root>/<peer_id>/` (config.json, logs/,
//! staging/, history_thumbs/). The active instance is resolved once at startup
//! (before logging) and stored process-global so legacy `config_path()` and the
//! derived `log_dir()`/`staging_root()`/`thumbs_root()` callers stay compatible.
//!
//! When the instance is not initialized (e.g. unit tests scoping via env vars),
//! paths fall back to the legacy flat layout under the platform data base.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::app_profile;

const APP_DATA_DIR_NAME: &str = "PlanarClip";
const CONFIG_FILE_NAME: &str = "config.json";
const HISTORY_THUMBS_DIR_NAME: &str = "history_thumbs";

/// Resolved paths for the active instance, keyed by peer id.
#[derive(Clone, Debug)]
pub struct InstancePaths {
    pub dir: PathBuf,
}

impl InstancePaths {
    pub fn new(peer_id: &str) -> Self {
        Self {
            dir: data_root().join(peer_id),
        }
    }

    pub fn config_path(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE_NAME)
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)
    }
}

static INSTANCE: Mutex<Option<InstancePaths>> = Mutex::new(None);

/// Install the active instance paths. Called once at the very start of `run()`.
pub fn set_instance(paths: InstancePaths) {
    *INSTANCE.lock().expect("instance paths lock poisoned") = Some(paths);
}

/// Reset the active instance (test-only).
#[cfg(test)]
pub fn reset_instance_for_test() {
    *INSTANCE.lock().expect("instance paths lock poisoned") = None;
}

fn instance_option() -> Option<InstancePaths> {
    INSTANCE
        .lock()
        .expect("instance paths lock poisoned")
        .clone()
}

/// Current config path: instance path when initialized, else legacy flat path.
pub fn current_config_path() -> PathBuf {
    match instance_option() {
        Some(p) => p.config_path(),
        None => legacy_config_path(),
    }
}

/// `<data_base>/PlanarClip/`.
pub fn data_root() -> PathBuf {
    platform_data_base().join(APP_DATA_DIR_NAME)
}

/// Legacy flat config path (pre-instance layout), used for migration reads and
/// as a fallback when the instance is not initialized.
pub fn legacy_config_path() -> PathBuf {
    platform_data_base().join(app_profile::CONFIG_FILE_NAME)
}

/// Scan a directory for instance subdirectories named by peer id.
/// Returns the most recently modified one, or `None` if there are none.
pub fn scan_instance_dirs(root: &Path) -> Option<String> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut dirs: Vec<(String, SystemTime)> = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_valid_peer_id_dir(&name) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        dirs.push((name, modified));
    }
    if dirs.is_empty() {
        return None;
    }
    // Most recently modified wins; `--instance` selection is a follow-up.
    dirs.sort_by(|a, b| b.1.cmp(&a.1));
    Some(dirs[0].0.clone())
}

/// A peer id is a 16-char lowercase hex string (blake3 prefix, see crypto/keys).
fn is_valid_peer_id_dir(name: &str) -> bool {
    name.len() == 16 && name.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// Copy legacy `history_thumbs/` into the instance directory during migration.
/// Staging and logs are not migrated (temporary / regenerated).
pub fn migrate_history_thumbs(legacy_parent: &Path, instance_dir: &Path) {
    let src = legacy_parent.join(HISTORY_THUMBS_DIR_NAME);
    if !src.is_dir() {
        return;
    }
    let dst = instance_dir.join(HISTORY_THUMBS_DIR_NAME);
    if std::fs::create_dir_all(&dst).is_err() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(&src) else {
        return;
    };
    for entry in entries.flatten() {
        let from = entry.path();
        if !from.is_file() {
            continue;
        }
        let to = dst.join(entry.file_name());
        let _ = std::fs::copy(&from, &to);
    }
}

fn platform_data_base() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support"))
            .unwrap_or_else(|_| PathBuf::from("."))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".config"))
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_id_dir_validates_16_lowercase_hex() {
        assert!(is_valid_peer_id_dir("0123456789abcdef"));
        assert!(!is_valid_peer_id_dir("0123456789ABCDEF")); // uppercase rejected
        assert!(!is_valid_peer_id_dir("short")); // wrong length
        assert!(!is_valid_peer_id_dir(&"g".repeat(16))); // non-hex
    }

    #[test]
    fn migrate_history_thumbs_copies_files() {
        let src_parent = tempfile::tempdir().unwrap();
        let src_thumbs = src_parent.path().join(HISTORY_THUMBS_DIR_NAME);
        std::fs::create_dir_all(&src_thumbs).unwrap();
        std::fs::write(src_thumbs.join("a.png"), b"data").unwrap();

        let instance = tempfile::tempdir().unwrap();
        migrate_history_thumbs(src_parent.path(), instance.path());

        assert!(instance
            .path()
            .join(HISTORY_THUMBS_DIR_NAME)
            .join("a.png")
            .exists());
    }

    #[test]
    fn migrate_history_thumbs_noop_when_missing() {
        let src_parent = tempfile::tempdir().unwrap();
        let instance = tempfile::tempdir().unwrap();
        // No history_thumbs under src -> no panic, nothing copied.
        migrate_history_thumbs(src_parent.path(), instance.path());
        assert!(!instance.path().join(HISTORY_THUMBS_DIR_NAME).exists());
    }
}
