use crate::clipboard::types::ClipboardHistoryEntry;
use crate::storage::json;
use std::path::PathBuf;

const HISTORY_FILE_NAME: &str = "history.json";

pub fn history_path() -> PathBuf {
    json::config_path()
        .parent()
        .map(|parent| parent.join(HISTORY_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(HISTORY_FILE_NAME))
}

pub fn load_history() -> Vec<ClipboardHistoryEntry> {
    let path = history_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn save_history(history: &[ClipboardHistoryEntry]) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(history) {
        let _ = std::fs::write(&path, json);
    }
}
