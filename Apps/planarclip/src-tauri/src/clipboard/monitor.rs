use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use tokio::sync::broadcast;

use crate::clipboard::types::{ClipboardEvent, ClipboardSnapshot};

static SELF_WRITING: AtomicBool = AtomicBool::new(false);
static SUPPRESSED_REMOTE_WRITE: Mutex<Option<SuppressedRemoteWrite>> = Mutex::new(None);
const CLIPBOARD_POLL_INTERVAL_MS: u64 = 150;
const REMOTE_WRITE_SUPPRESSION_MS: u64 = 1_500;

struct SuppressedRemoteWrite {
    hash: [u8; 32],
    until_ms: u64,
}

pub struct ClipboardMonitor {
    tx: broadcast::Sender<ClipboardEvent>,
    last_hash: [u8; 32],
    last_read_error: Option<String>,
}

impl ClipboardMonitor {
    pub fn new(tx: broadcast::Sender<ClipboardEvent>) -> Self {
        Self {
            tx,
            last_hash: [0u8; 32],
            last_read_error: None,
        }
    }

    pub fn set_self_writing(flag: bool) {
        SELF_WRITING.store(flag, Ordering::SeqCst);
    }

    pub fn is_self_writing() -> bool {
        SELF_WRITING.load(Ordering::SeqCst)
    }

    pub async fn run(&mut self) {
        self.run_polling_loop().await;
    }

    async fn run_polling_loop(&mut self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(CLIPBOARD_POLL_INTERVAL_MS));
        loop {
            interval.tick().await;
            self.capture_clipboard_change();
        }
    }

    fn capture_clipboard_change(&mut self) {
        if Self::is_self_writing() {
            return;
        }

        match Self::read_clipboard() {
            Ok(snapshot) => {
                self.last_read_error.take();

                let hash = snapshot.content_hash();
                if Self::should_suppress_local_emit(hash) {
                    self.last_hash = hash;
                    return;
                }

                if hash != self.last_hash {
                    self.last_hash = hash;
                    let _ = self.tx.send(ClipboardEvent::local(snapshot));
                }
            }
            Err(error) => {
                if self.last_read_error.as_deref() != Some(error.as_str()) {
                    self.last_read_error = Some(error.clone());
                }
            }
        }
    }

    fn read_clipboard() -> Result<ClipboardSnapshot, String> {
        let mut clipboard = arboard::Clipboard::new().map_err(|error| format!("clipboard init failed: {error}"))?;
        let text = clipboard
            .get_text()
            .map_err(|error| format!("clipboard read failed: {error}"))?;

        Ok(ClipboardSnapshot::Text(text))
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
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
