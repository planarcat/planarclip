use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;

use crate::clipboard::types::{debug_report, ClipboardEvent, ClipboardSnapshot};

static SELF_WRITING: AtomicBool = AtomicBool::new(false);

pub struct ClipboardMonitor {
    tx: broadcast::Sender<ClipboardEvent>,
    last_hash: [u8; 32],
}

impl ClipboardMonitor {
    pub fn new(tx: broadcast::Sender<ClipboardEvent>) -> Self {
        Self {
            tx,
            last_hash: [0u8; 32],
        }
    }

    pub fn set_self_writing(flag: bool) {
        SELF_WRITING.store(flag, Ordering::SeqCst);
    }

    pub fn is_self_writing() -> bool {
        SELF_WRITING.load(Ordering::SeqCst)
    }

    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            interval.tick().await;
            if Self::is_self_writing() {
                continue;
            }
            if let Some(snapshot) = Self::read_clipboard() {
                let hash = snapshot.content_hash();
                if hash != self.last_hash {
                    self.last_hash = hash;
                    // #region debug-point A:monitor-local-emit
                    debug_report(
                        "A",
                        "clipboard/monitor.rs:36",
                        "[DEBUG] monitor emitted local clipboard event",
                        serde_json::json!({
                            "hash": hex::encode(hash),
                            "self_writing": Self::is_self_writing(),
                            "text_len": snapshot.text().map(|text| text.len()).unwrap_or(0),
                        }),
                    );
                    // #endregion
                    let _ = self.tx.send(ClipboardEvent::local(snapshot));
                }
            }
        }
    }

    fn read_clipboard() -> Option<ClipboardSnapshot> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        clipboard
            .get_text()
            .ok()
            .map(ClipboardSnapshot::Text)
            .or(Some(ClipboardSnapshot::Empty))
    }

    pub fn write_clipboard(text: &str) {
        // #region debug-point A:remote-write-start
        debug_report(
            "A",
            "clipboard/monitor.rs:60",
            "[DEBUG] write_clipboard started",
            serde_json::json!({
                "hash": hex::encode(blake3::hash(text.as_bytes()).as_bytes()),
                "text_len": text.len(),
            }),
        );
        // #endregion
        Self::set_self_writing(true);
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
        Self::set_self_writing(false);
        // #region debug-point A:remote-write-end
        debug_report(
            "A",
            "clipboard/monitor.rs:73",
            "[DEBUG] write_clipboard finished",
            serde_json::json!({
                "hash": hex::encode(blake3::hash(text.as_bytes()).as_bytes()),
                "self_writing": Self::is_self_writing(),
            }),
        );
        // #endregion
    }
}
