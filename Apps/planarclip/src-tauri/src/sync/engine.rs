use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use crate::clipboard::types::ClipboardSnapshot;
use crate::network::webrtc::ConnectionHandle;

pub struct SyncEngine {
    rx: broadcast::Receiver<ClipboardSnapshot>,
    connection: Arc<Mutex<Option<ConnectionHandle>>>,
}

impl SyncEngine {
    pub fn new(
        rx: broadcast::Receiver<ClipboardSnapshot>,
        connection: Arc<Mutex<Option<ConnectionHandle>>>,
    ) -> Self {
        Self { rx, connection }
    }

    pub async fn run(mut self) {
        loop {
            match self.rx.recv().await {
                Ok(snapshot) => {
                    let conn = self.connection.lock().await;
                    if let Some(ref handle) = *conn {
                        handle.send_clipboard(&snapshot);
                    }
                }
                Err(e) => {
                    tracing::error!("Sync engine channel error: {}", e);
                    break;
                }
            }
        }
    }
}
