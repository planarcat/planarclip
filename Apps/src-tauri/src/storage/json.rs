use crate::clipboard::types::ClipboardHistoryEntry;
use crate::storage::paths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub device_name: String,
    pub key_pair: Option<KeyPairData>,
    pub paired_peer: Option<PeerData>,
    pub tcp_port: Option<u16>,
    pub lan_enabled: Option<bool>,
    pub trusted_peers: Option<Vec<TrustedPeerData>>,
    pub ui_color_scheme: Option<String>,
    pub ui_theme_color: Option<String>,
    /// Legacy field kept only to migrate old configs into `history.json`; new writes go to history.json.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clipboard_history: Option<Vec<ClipboardHistoryEntry>>,
    pub launch_at_startup: Option<bool>,
    pub silent_start: Option<bool>,
    /// When true (default), show OS toasts for connection/sync events.
    pub system_notifications_enabled: Option<bool>,
    /// `"tray"` (default) or `"exit"` when the user closes the main window.
    pub close_window_action: Option<String>,
    pub auto_connect_trusted: Option<bool>,
    /// When true (default), local clipboard changes are synced automatically.
    pub auto_sync_clipboard: Option<bool>,
    pub sync_images: Option<bool>,
    pub sync_files: Option<bool>,
    /// When true, received synced files are copied to `sync_files_save_dir` (or Downloads).
    pub sync_files_save_enabled: Option<bool>,
    /// Custom directory for received synced files; `None` uses the system Downloads folder.
    pub sync_files_save_dir: Option<String>,
    pub max_file_bytes: Option<u64>,
    pub clipboard_history_limit: Option<usize>,
    /// Clipboard history UI layout: `"list"` or `"grid"` (default grid when unset).
    pub clipboard_view_mode: Option<String>,
    /// When true, raise the log level to debug for diagnostics (runtime + persisted).
    pub verbose_log: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyPairData {
    pub secret_bytes: Vec<u8>,
    pub public_bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerData {
    pub name: String,
    pub public_key: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrustedPeerData {
    pub name: String,
    pub public_key: Vec<u8>,
    pub peer_id: String,
    pub last_ip: Option<String>,
    #[serde(default)]
    pub auto_accept: Option<bool>,
}

pub fn config_path() -> PathBuf {
    paths::current_config_path()
}

#[allow(dead_code)]
pub fn load_config() -> AppConfig {
    load_config_at(&config_path())
}

pub fn load_config_at(path: &Path) -> AppConfig {
    if path.exists() {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

pub fn save_config(config: &AppConfig) {
    save_config_at(config, &config_path());
}

pub fn save_config_at(config: &AppConfig, path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(path, json);
    }
}


// ---- inline unit tests ----
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // config_path() 依赖平台专用环境变量（Windows: APPDATA, macOS/Linux: HOME）。
    // 修改环境变量在测试线程间共享，用互斥锁避免并发用例互相干扰。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_key() -> &'static str {
        if cfg!(target_os = "windows") {
            "APPDATA"
        } else {
            "HOME"
        }
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(dir: &std::path::Path) -> (Self, std::sync::MutexGuard<'static, ()>) {
            let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let key = env_key();
            let previous = std::env::var(key).ok();
            std::env::set_var(key, dir);
            (Self { key, previous }, guard)
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn app_config_default_has_empty_optionals() {
        let cfg = AppConfig::default();
        assert!(cfg.device_name.is_empty());
        assert!(cfg.key_pair.is_none());
        assert!(cfg.trusted_peers.is_none());
        assert!(cfg.tcp_port.is_none());
    }

    #[test]
    fn load_config_returns_default_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, _lock) = EnvGuard::set(dir.path());
        // 目录内无 CONFIG_FILE_NAME 时应回退到默认值，不 panic
        let cfg = load_config();
        assert!(cfg.device_name.is_empty());
    }

    #[test]
    fn load_config_returns_default_when_file_is_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, _lock) = EnvGuard::set(dir.path());
        let path = config_path();
        std::fs::write(&path, "{ not valid json").unwrap();
        // 损坏的 JSON 必须回退默认，绝不能 panic 影响应用启动
        let cfg = load_config();
        assert!(cfg.device_name.is_empty());
        assert!(cfg.trusted_peers.is_none());
    }

    #[test]
    fn save_then_load_roundtrip_preserves_fields() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, _lock) = EnvGuard::set(dir.path());
        let mut cfg = AppConfig::default();
        cfg.device_name = "planarcat-win11".into();
        cfg.tcp_port = Some(19876);
        cfg.trusted_peers = Some(vec![TrustedPeerData {
            name: "A".into(),
            public_key: vec![1, 2, 3, 4],
            peer_id: "abcd".into(),
            last_ip: Some("192.168.0.10".into()),
            auto_accept: Some(true),
        }]);
        save_config(&cfg);

        let loaded = load_config();
        assert_eq!(loaded.device_name, "planarcat-win11");
        assert_eq!(loaded.tcp_port, Some(19876));
        let peers = loaded.trusted_peers.expect("trusted_peers preserved");
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "abcd");
        assert_eq!(peers[0].auto_accept, Some(true));
    }

    #[test]
    fn trusted_peer_auto_accept_defaults_to_none_when_missing() {
        // 老版本配置里没有 auto_accept 字段时，反序列化后必须为 None
        let json = r#"[{"name":"A","public_key":[1,2],"peer_id":"pa","last_ip":null}]"#;
        let peers: Vec<TrustedPeerData> = serde_json::from_str(json).unwrap();
        assert_eq!(peers.len(), 1);
        assert!(peers[0].auto_accept.is_none());
    }
}
