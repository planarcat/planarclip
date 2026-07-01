#[derive(Debug, Clone)]
pub enum ClipboardSnapshot {
    Text(String),
    Image {
        png_bytes: Vec<u8>,
        width: u32,
        height: u32,
    },
    FileList {
        files: Vec<ClipboardFileItem>,
    },
    Empty,
}

#[derive(Debug, Clone)]
pub struct ClipboardFileItem {
    pub file_name: String,
    pub size_bytes: u64,
    pub content_hash: [u8; 32],
    pub source_path: Option<std::path::PathBuf>,
}

impl ClipboardSnapshot {
    pub fn content_hash(&self) -> [u8; 32] {
        match self {
            ClipboardSnapshot::Text(s) => *blake3::hash(s.as_bytes()).as_bytes(),
            ClipboardSnapshot::Image { png_bytes, .. } => *blake3::hash(png_bytes).as_bytes(),
            ClipboardSnapshot::FileList { files } => crate::clipboard::file::file_list_hash(files),
            ClipboardSnapshot::Empty => [0u8; 32],
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            ClipboardSnapshot::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, ClipboardSnapshot::Empty)
    }
}

#[derive(Debug, Clone)]
pub enum ClipboardOrigin {
    Local,
    Remote { peer_name: String },
}

#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    pub snapshot: ClipboardSnapshot,
    pub origin: ClipboardOrigin,
    pub timestamp_ms: u64,
    /// When true, history merge is skipped (e.g. manual send already updated history).
    pub skip_history_merge: bool,
}

impl ClipboardEvent {
    pub fn local(snapshot: ClipboardSnapshot) -> Self {
        Self {
            snapshot,
            origin: ClipboardOrigin::Local,
            timestamp_ms: now_ms(),
            skip_history_merge: false,
        }
    }

    pub fn local_sync_only(snapshot: ClipboardSnapshot) -> Self {
        Self {
            snapshot,
            origin: ClipboardOrigin::Local,
            timestamp_ms: now_ms(),
            skip_history_merge: true,
        }
    }

    pub fn remote(snapshot: ClipboardSnapshot, peer_name: String) -> Self {
        Self {
            snapshot,
            origin: ClipboardOrigin::Remote { peer_name },
            timestamp_ms: now_ms(),
            skip_history_merge: false,
        }
    }

    pub fn content_hash(&self) -> [u8; 32] {
        self.snapshot.content_hash()
    }
}

fn default_clip_type() -> String {
    "text".to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClipboardHistoryEntry {
    pub id: String,
    pub content: String,
    #[serde(default = "default_clip_type")]
    pub clip_type: String,
    pub source_label: String,
    pub direction: String,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub size_label: Option<String>,
    #[serde(default)]
    pub image_data_url: Option<String>,
    #[serde(default)]
    pub file_count: Option<u32>,
    #[serde(default)]
    pub file_names: Option<Vec<String>>,
    #[serde(default)]
    pub preview_kind: Option<String>,
    #[serde(default)]
    pub thumbnail_ref: Option<String>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
