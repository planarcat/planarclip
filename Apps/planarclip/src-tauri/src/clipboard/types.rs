/// Represents the current clipboard content
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
}
