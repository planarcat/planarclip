use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::{broadcast, Mutex};

use crate::clipboard::image::INLINE_IMAGE_BYTES;
use crate::clipboard::types::{ClipboardEvent, ClipboardOrigin, ClipboardSnapshot};
use crate::network::webrtc::ConnectionHandle;
use crate::storage::json::AppConfig;

pub struct SyncEngine {
    rx: broadcast::Receiver<ClipboardEvent>,
    connection: Arc<Mutex<Option<ConnectionHandle>>>,
    config: Arc<Mutex<AppConfig>>,
    app_handle: AppHandle,
}

impl SyncEngine {
    pub fn new(
        rx: broadcast::Receiver<ClipboardEvent>,
        connection: Arc<Mutex<Option<ConnectionHandle>>>,
        config: Arc<Mutex<AppConfig>>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            rx,
            connection,
            config,
            app_handle,
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if matches!(event.origin, ClipboardOrigin::Remote { .. }) {
                        continue;
                    }

                    let sync_images = {
                        let config = self.config.lock().await;
                        config.sync_images.unwrap_or(true)
                    };

                    let needs_chunked = matches!(
                        &event.snapshot,
                        ClipboardSnapshot::Image { png_bytes, .. }
                            if png_bytes.len() > INLINE_IMAGE_BYTES
                    );

                    let conn = self.connection.lock().await;
                    let Some(ref handle) = *conn else {
                        continue;
                    };

                    let is_connected = handle
                        .connected()
                        .try_lock()
                        .map(|guard| *guard)
                        .unwrap_or(false);
                    if !is_connected {
                        continue;
                    }

                    if needs_chunked && handle.supports_chunked_images() {
                        let handle = handle.clone();
                        let snapshot = event.snapshot.clone();
                        let app_handle = self.app_handle.clone();
                        drop(conn);
                        tokio::spawn(async move {
                            handle
                                .send_image_async(snapshot, sync_images, Some(app_handle))
                                .await;
                        });
                    } else {
                        handle.send_snapshot(
                            &event.snapshot,
                            sync_images,
                            Some(&self.app_handle),
                        );
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
