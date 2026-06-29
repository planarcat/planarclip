use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::{broadcast, Mutex};

use crate::clipboard::file::{image_snapshot_from_single_file, DEFAULT_MAX_FILE_BYTES};
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

                    let (sync_images, sync_files, max_file_bytes) = {
                        let config = self.config.lock().await;
                        (
                            config.sync_images.unwrap_or(true),
                            config.sync_files.unwrap_or(false),
                            config
                                .max_file_bytes
                                .unwrap_or(DEFAULT_MAX_FILE_BYTES),
                        )
                    };

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

                    match &event.snapshot {
                        ClipboardSnapshot::Image { png_bytes, .. }
                            if png_bytes.len() > INLINE_IMAGE_BYTES
                                && handle.supports_chunked_images() =>
                        {
                            let handle = handle.clone();
                            let snapshot = event.snapshot.clone();
                            let app_handle = self.app_handle.clone();
                            drop(conn);
                            tokio::spawn(async move {
                                handle
                                    .send_image_async(snapshot, sync_images, Some(app_handle))
                                    .await;
                            });
                        }
                        ClipboardSnapshot::FileList { files } if sync_files => {
                            let handle = handle.clone();
                            let snapshot = event.snapshot.clone();
                            let app_handle = self.app_handle.clone();
                            drop(conn);
                            tokio::spawn(async move {
                                handle
                                    .send_files_async(
                                        snapshot,
                                        true,
                                        max_file_bytes,
                                        Some(app_handle),
                                    )
                                    .await;
                            });
                        }
                        ClipboardSnapshot::FileList { files } => {
                            let handle = handle.clone();
                            let snapshot = event.snapshot.clone();
                            let app_handle = self.app_handle.clone();
                            let image_snapshot = sync_images
                                .then(|| image_snapshot_from_single_file(files))
                                .flatten();
                            let needs_chunked = image_snapshot.as_ref().is_some_and(|snapshot| {
                                matches!(
                                    snapshot,
                                    ClipboardSnapshot::Image { png_bytes, .. }
                                        if png_bytes.len() > INLINE_IMAGE_BYTES
                                )
                            });
                            drop(conn);
                            if image_snapshot.is_none() {
                                handle.send_file_list_meta(&snapshot, Some(&app_handle));
                            }
                            if let Some(image_snapshot) = image_snapshot {
                                if needs_chunked && handle.supports_chunked_images() {
                                    tokio::spawn(async move {
                                        handle
                                            .send_image_async(image_snapshot, true, Some(app_handle))
                                            .await;
                                    });
                                } else {
                                    handle.send_snapshot(&image_snapshot, true, Some(&app_handle));
                                }
                            }
                        }
                        _ => {
                            handle.send_snapshot(
                                &event.snapshot,
                                sync_images,
                                Some(&self.app_handle),
                            );
                        }
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
