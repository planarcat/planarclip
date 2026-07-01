use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::{broadcast, Mutex};

use crate::clipboard::file::DEFAULT_MAX_FILE_BYTES;
use crate::clipboard::image::INLINE_IMAGE_BYTES;
use crate::clipboard::types::{ClipboardEvent, ClipboardOrigin, ClipboardSnapshot};
use crate::network::sessions::ConnectionRegistry;
use crate::network::webrtc::ConnectionHandle;
use crate::storage::json::AppConfig;
use crate::sync::transfer_limit::TransferSlotLimiter;

pub struct SyncEngine {
    rx: broadcast::Receiver<ClipboardEvent>,
    connections: Arc<Mutex<ConnectionRegistry>>,
    transfer_slots: Arc<TransferSlotLimiter>,
    config: Arc<Mutex<AppConfig>>,
    app_handle: AppHandle,
}

impl SyncEngine {
    pub fn new(
        rx: broadcast::Receiver<ClipboardEvent>,
        connections: Arc<Mutex<ConnectionRegistry>>,
        transfer_slots: Arc<TransferSlotLimiter>,
        config: Arc<Mutex<AppConfig>>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            rx,
            connections,
            transfer_slots,
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

                    let (sync_images, sync_files, max_file_bytes, auto_sync_clipboard) = {
                        let config = self.config.lock().await;
                        (
                            config.sync_images.unwrap_or(true),
                            config.sync_files.unwrap_or(true),
                            config
                                .max_file_bytes
                                .unwrap_or(DEFAULT_MAX_FILE_BYTES),
                            config.auto_sync_clipboard.unwrap_or(true),
                        )
                    };

                    if !auto_sync_clipboard && !event.skip_history_merge {
                        continue;
                    }

                    let handles = {
                        let registry = self.connections.lock().await;
                        registry.active_handles()
                    };

                    if handles.is_empty() {
                        continue;
                    }

                    Self::broadcast_snapshot(
                        &handles,
                        &event.snapshot,
                        sync_images,
                        sync_files,
                        max_file_bytes,
                        self.transfer_slots.clone(),
                        self.app_handle.clone(),
                    );
                }
                Err(e) => {
                    tracing::error!("Sync engine channel error: {}", e);
                    break;
                }
            }
        }
    }

    fn broadcast_snapshot(
        handles: &[ConnectionHandle],
        snapshot: &ClipboardSnapshot,
        sync_images: bool,
        sync_files: bool,
        max_file_bytes: u64,
        transfer_slots: Arc<TransferSlotLimiter>,
        app_handle: AppHandle,
    ) {
        for handle in handles {
            match snapshot {
                ClipboardSnapshot::Image { png_bytes, .. }
                    if png_bytes.len() > INLINE_IMAGE_BYTES
                        && handle.supports_chunked_images() =>
                {
                    let handle = handle.clone();
                    let snapshot = snapshot.clone();
                    let slots = transfer_slots.clone();
                    let app = app_handle.clone();
                    tokio::spawn(async move {
                        let _permit = slots.acquire().await;
                        handle
                            .send_image_async(snapshot, sync_images, Some(app))
                            .await;
                    });
                }
                ClipboardSnapshot::FileList { .. } if sync_files => {
                    let handle = handle.clone();
                    let snapshot = snapshot.clone();
                    let slots = transfer_slots.clone();
                    let app = app_handle.clone();
                    tokio::spawn(async move {
                        let _permit = slots.acquire().await;
                        handle
                            .send_files_async(snapshot, true, max_file_bytes, Some(app))
                            .await;
                    });
                }
                _ => {
                    handle.send_snapshot(snapshot, sync_images, Some(&app_handle));
                }
            }
        }
    }
}
