//! Per-instance data directory resolution.
//!
//! The instance id lives in the **install directory** (`<exe_dir>/config.json`,
//! `{ "id": "<peer_id>" }`), set on first launch. App data lives under
//! `<data_root>/<id>/` (config.json, logs/, staging/, history_media/, ...).
//! Multiple install directories (dev build, copied prod, etc.) each get their
//! own id and data directory -- no scanning, no accidental sharing.

use std::path::PathBuf;
use std::sync::Mutex;

const APP_DATA_DIR_NAME: &str = "PlanarClip";
const CONFIG_FILE_NAME: &str = "config.json";

/// Resolved paths for the active instance, keyed by id.
#[derive(Clone, Debug)]
pub struct InstancePaths {
    pub dir: PathBuf,
}

impl InstancePaths {
    pub fn new(id: &str) -> Self {
        Self {
            dir: data_root().join(id),
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

/// Current config path: instance path when initialized, else a flat fallback.
pub fn current_config_path() -> PathBuf {
    match instance_option() {
        Some(p) => p.config_path(),
        None => platform_data_base().join(CONFIG_FILE_NAME),
    }
}

/// `<data_base>/PlanarClip/` -- shared root; each instance lives under `<id>/`.
pub fn data_root() -> PathBuf {
    platform_data_base().join(APP_DATA_DIR_NAME)
}

/// Path to the install-directory config (`<exe_dir>/config.json`) holding the
/// instance id. `None` if the exe path can't be resolved.
pub fn install_config_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    Some(dir.join(CONFIG_FILE_NAME))
}

#[derive(serde::Deserialize)]
struct InstallConfig {
    #[serde(default)]
    id: Option<String>,
}

/// Read the instance id from the install-directory config.
pub fn read_install_id() -> Option<String> {
    let path = install_config_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let cfg: InstallConfig = serde_json::from_str(&content).ok()?;
    cfg.id.filter(|s| !s.is_empty())
}

/// Write the instance id into the install-directory config (first launch).
pub fn write_install_id(id: &str) {
    let Some(path) = install_config_path() else {
        return;
    };
    let json = format!("{{\"id\":\"{id}\"}}");
    let _ = std::fs::write(&path, json);
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
    fn instance_paths_join_id() {
        let p = InstancePaths::new("abc123");
        assert!(p.dir.ends_with("abc123"));
        assert!(p.config_path().ends_with(CONFIG_FILE_NAME));
    }
}
