use crate::app_profile::CONFIG_FILE_NAME;
use crate::clipboard::types::ClipboardHistoryEntry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
    path.push(CONFIG_FILE_NAME);
    path
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join("Library/Application Support"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config"))
    }
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

pub fn save_config(config: &AppConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(&path, json);
    }
}
