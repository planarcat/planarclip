use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tokio::sync::broadcast;

use crate::clipboard::types::{debug_report, ClipboardEvent, ClipboardSnapshot};

static SELF_WRITING: AtomicBool = AtomicBool::new(false);
static SUPPRESSED_REMOTE_WRITE: Mutex<Option<SuppressedRemoteWrite>> = Mutex::new(None);
const REMOTE_WRITE_SUPPRESSION_MS: u64 = 1_500;

struct SuppressedRemoteWrite {
    hash: [u8; 32],
    until_ms: u64,
}

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
                if Self::should_suppress_local_emit(hash) {
                    self.last_hash = hash;
                    // #region debug-point A:monitor-local-suppressed
                    debug_report(
                        "A",
                        "clipboard/monitor.rs:42",
                        "[DEBUG] monitor suppressed local clipboard echo",
                        serde_json::json!({
                            "hash": hex::encode(hash),
                            "text_len": snapshot.text().map(|text| text.len()).unwrap_or(0),
                        }),
                    );
                    // #endregion
                    continue;
                }
                if hash != self.last_hash {
                    self.last_hash = hash;
                    // #region debug-point A:monitor-local-emit
                    debug_report(
                        "A",
                        "clipboard/monitor.rs:54",
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
        clipboard.get_text().ok().map(ClipboardSnapshot::Text)
    }

    fn should_suppress_local_emit(hash: [u8; 32]) -> bool {
        let now_ms = now_ms();
        let Ok(mut state) = SUPPRESSED_REMOTE_WRITE.lock() else {
            return false;
        };

        match state.as_ref() {
            Some(suppressed) if suppressed.until_ms >= now_ms && suppressed.hash == hash => true,
            Some(suppressed) if suppressed.until_ms < now_ms => {
                *state = None;
                false
            }
            _ => false,
        }
    }

    pub fn write_clipboard(text: &str) {
        let hash = *blake3::hash(text.as_bytes()).as_bytes();
        // #region debug-point A:remote-write-start
        debug_report(
            "A",
            "clipboard/monitor.rs:93",
            "[DEBUG] write_clipboard started",
            serde_json::json!({
                "hash": hex::encode(hash),
                "text_len": text.len(),
            }),
        );
        // #endregion
        if let Ok(mut state) = SUPPRESSED_REMOTE_WRITE.lock() {
            *state = Some(SuppressedRemoteWrite {
                hash,
                until_ms: now_ms() + REMOTE_WRITE_SUPPRESSION_MS,
            });
        }
        Self::set_self_writing(true);
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
        Self::set_self_writing(false);
        // #region debug-point A:remote-write-end
        debug_report(
            "A",
            "clipboard/monitor.rs:112",
            "[DEBUG] write_clipboard finished",
            serde_json::json!({
                "hash": hex::encode(hash),
                "self_writing": Self::is_self_writing(),
                "suppression_until_ms": now_ms() + REMOTE_WRITE_SUPPRESSION_MS,
            }),
        );
        // #endregion
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
