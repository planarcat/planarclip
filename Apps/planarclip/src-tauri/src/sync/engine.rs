use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use crate::clipboard::types::ClipboardSnapshot;

pub struct SyncEngine {
    rx: broadcast::Receiver<ClipboardSnapshot>,
    connected: Arc<Mutex<bool>>,
}

impl SyncEngine {
    pub fn new(
        rx: broadcast::Receiver<ClipboardSnapshot>,
        connected: Arc<Mutex<bool>>,
    ) -> Self {
        Self { rx, connected }
    }

    pub async fn run(mut self) {
        loop {
            match self.rx.recv().await {
                Ok(snapshot) => {
                    let is_connected = *self.connected.lock().await;
                    if !is_connected {
                        tracing::info!("Not connected, skipping sync");
                        continue;
                    }
                    tracing::info!("Clipboard changed: {:?}", snapshot);
                    // TODO: broadcast to connected peers via WebRTC
                }
                Err(e) => {
                    tracing::error!("Sync engine error: {}", e);
                }
            }
        }
    }
}
