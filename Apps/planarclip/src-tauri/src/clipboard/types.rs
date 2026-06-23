#[derive(Debug, Clone)]
pub enum ClipboardSnapshot {
    Text(String),
    Empty,
}

impl ClipboardSnapshot {
    pub fn content_hash(&self) -> [u8; 32] {
        match self {
            ClipboardSnapshot::Text(s) => *blake3::hash(s.as_bytes()).as_bytes(),
            ClipboardSnapshot::Empty => [0u8; 32],
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            ClipboardSnapshot::Text(text) => Some(text),
            ClipboardSnapshot::Empty => None,
        }
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
}

impl ClipboardEvent {
    pub fn local(snapshot: ClipboardSnapshot) -> Self {
        Self {
            snapshot,
            origin: ClipboardOrigin::Local,
            timestamp_ms: now_ms(),
        }
    }

    pub fn remote(snapshot: ClipboardSnapshot, peer_name: String) -> Self {
        Self {
            snapshot,
            origin: ClipboardOrigin::Remote { peer_name },
            timestamp_ms: now_ms(),
        }
    }

    pub fn content_hash(&self) -> [u8; 32] {
        self.snapshot.content_hash()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClipboardHistoryEntry {
    pub id: String,
    pub content: String,
    pub source_label: String,
    pub direction: String,
    pub timestamp_ms: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
