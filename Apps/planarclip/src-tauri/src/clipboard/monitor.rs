use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;

use crate::clipboard::types::{ClipboardEvent, ClipboardSnapshot};

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
        Self::set_self_writing(true);
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
        Self::set_self_writing(false);
    }
}
