use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::clipboard::file::{file_list_hash, file_list_summary, FILE_TRANSFER_LIMIT_MESSAGE, MAX_BATCH_BYTES, SYNC_NOT_CONNECTED_MESSAGE};
use crate::clipboard::image::{format_byte_size, INLINE_IMAGE_BYTES, MAX_IMAGE_BYTES};
use crate::clipboard::monitor::ClipboardMonitor;
use crate::clipboard::types::{ClipboardEvent, ClipboardFileItem, ClipboardSnapshot};
use crate::network::binary_chunk::BinaryChunk;
use crate::network::sessions::{ConnectionRegistry, ConnectionSession};
use crate::network::direct::{ConnectionEvent, DirectConnection};
use crate::network::protocol::SignalMessage;
use crate::network::signalling;
use crate::sync::activity::{emit_sync_activity, notify_sync_failure, TransferProgressReporter};
use crate::sync::dedup::DedupStore;
use crate::sync::transfer::{
    cancel_transfer_ack, max_file_bytes_to_mb, peer_file_too_large_message,
    peer_file_receive_declined_message, peer_image_receive_declined_message,
    peer_image_too_large_message, route_transfer_ack,
    send_file_with_flow_control, send_image_with_flow_control, CompletedFile,
    CompletedImage, FileReceiveSession, ImageReceiveSession, PEER_TRANSFER_HANDLED,
};

#[derive(Clone)]
enum HandleTransport {
    Direct {
        tx: mpsc::UnboundedSender<ConnectionEvent>,
    },
    Signalling {
        tx: mpsc::UnboundedSender<String>,
    },
}

#[derive(Clone)]
pub struct ConnectionHandle {
    transport: HandleTransport,
    connected: Arc<Mutex<bool>>,
    dedup: Arc<Mutex<DedupStore>>,
    ack_waiters: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<u32>>>>,
    transfer_cancel_reasons: Arc<Mutex<HashMap<String, String>>>,
}

impl ConnectionHandle {
    pub fn connected(&self) -> Arc<Mutex<bool>> {
        self.connected.clone()
    }

    pub fn supports_chunked_images(&self) -> bool {
        matches!(self.transport, HandleTransport::Direct { .. })
    }

    pub fn supports_chunked_files(&self) -> bool {
        self.supports_chunked_images()
    }

    pub fn notify_peer_left(&self, local_peer_id: &str) {
        self.send_signal(SignalMessage::PeerLeft {
            peer_id: local_peer_id.to_string(),
        });
    }

    fn send_signal(&self, msg: SignalMessage) {
        match &self.transport {
            HandleTransport::Direct { tx } => {
                let _ = tx.send(ConnectionEvent::Signal(msg));
            }
            HandleTransport::Signalling { tx } => {
                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = tx.send(json);
                }
            }
        }
    }

    fn send_binary(&self, chunk: BinaryChunk) {
        if let HandleTransport::Direct { tx } = &self.transport {
            let _ = tx.send(ConnectionEvent::Binary(chunk));
        }
    }

    pub fn send_snapshot(
        &self,
        snapshot: &ClipboardSnapshot,
        sync_images: bool,
        app_handle: Option<&AppHandle>,
    ) {
        let is_connected = self
            .connected
            .try_lock()
            .map(|guard| *guard)
            .unwrap_or(false);
        if !is_connected {
            return;
        }

        match snapshot {
            ClipboardSnapshot::Text(text) => self.send_text(text),
            ClipboardSnapshot::Image {
                png_bytes,
                width,
                height,
            } => {
                if !sync_images {
                    return;
                }

                let size = png_bytes.len();
                if size > MAX_IMAGE_BYTES {
                    notify_sync_failure(app_handle, "图片超过 5 MB，未同步到其他设备。");
                    return;
                }

                if size > INLINE_IMAGE_BYTES {
                    if !self.supports_chunked_images() {
                        notify_sync_failure(
                            app_handle,
                            "图片较大，当前连接方式暂不支持同步超过 512 KB 的图片。",
                        );
                    }
                    // Chunked sends are handled by send_image_async from SyncEngine.
                    return;
                }

                self.send_image_inline(png_bytes, *width, *height, app_handle);
            }
            ClipboardSnapshot::Empty => {}
            ClipboardSnapshot::FileList { .. } => {
                // Full file transfer is handled by SyncEngine; disabled sync uses Text snapshot.
            }
        }
    }

    pub async fn send_files_async(
        &self,
        snapshot: ClipboardSnapshot,
        sync_files: bool,
        max_file_bytes: u64,
        app_handle: Option<AppHandle>,
    ) {
        let ClipboardSnapshot::FileList { files } = snapshot else {
            return;
        };

        if !sync_files || files.is_empty() {
            return;
        }

        let is_connected = *self.connected.lock().await;
        if !is_connected {
            notify_sync_failure(
                app_handle.as_ref(),
                SYNC_NOT_CONNECTED_MESSAGE,
            );
            return;
        }

        if !self.supports_chunked_files() {
            notify_sync_failure(
                app_handle.as_ref(),
                "文件同步需要局域网直连，当前连接方式暂不支持。",
            );
            return;
        }

        let batch_bytes: u64 = files.iter().map(|file| file.size_bytes).sum();
        if batch_bytes > MAX_BATCH_BYTES {
            notify_sync_failure(
                app_handle.as_ref(),
                FILE_TRANSFER_LIMIT_MESSAGE,
            );
            return;
        }

        for file in &files {
            if file.size_bytes > max_file_bytes {
                notify_sync_failure(
                    app_handle.as_ref(),
                    FILE_TRANSFER_LIMIT_MESSAGE,
                );
                return;
            }
        }

        let batch_id = if files.len() > 1 {
            Some(uuid::Uuid::new_v4().to_string())
        } else {
            None
        };
        let batch_total = files.len() as u32;

        let mut progress = TransferProgressReporter::file_send(
            app_handle.clone(),
            files
                .first()
                .map(|file| file.file_name.clone())
                .unwrap_or_else(|| "文件".to_string()),
            None,
            batch_id.as_ref().map(|_| batch_total),
            batch_bytes,
            0,
        );

        let mut failed = false;
        let mut last_send_error = String::new();

        for (index, file) in files.iter().enumerate() {
            progress.set_label(file.file_name.clone());
            if batch_id.is_some() {
                progress.set_batch_index(Some(index as u32));
            }

            let Some(source_path) = file.source_path.as_ref() else {
                failed = true;
                last_send_error = "文件同步失败，请重新复制文件后再试。".to_string();
                break;
            };

            let file_size = file.size_bytes;
            let progress_sink = |acked: u32, chunk_total: u32| {
                let force = acked == 0 || acked >= chunk_total;
                progress.report_chunks(acked, chunk_total, file_size, force);
            };

            let result = send_file_with_flow_control(
                |msg| self.send_signal(msg),
                |chunk| self.send_binary(chunk),
                &self.ack_waiters,
                &self.transfer_cancel_reasons,
                source_path,
                &file.file_name,
                file.content_hash,
                batch_id.clone(),
                batch_id.as_ref().map(|_| index as u32),
                batch_id.as_ref().map(|_| batch_total),
                Some(progress_sink),
            )
            .await;

            if let Err(error) = result {
                failed = true;
                last_send_error = error;
                tracing::warn!("File send failed for {}: {last_send_error}", file.file_name);
                break;
            }

            progress.complete_file_in_batch(file.size_bytes);
        }

        if !failed {
            if let Some(batch_id) = batch_id {
                self.send_signal(SignalMessage::ClipboardFileBatchEnd {
                    batch_id,
                    file_count: batch_total,
                });
            }
            progress.finish("文件已同步");
        } else if !last_send_error.is_empty() {
            notify_sync_failure(app_handle.as_ref(), &last_send_error);
        }
    }

    pub async fn send_image_async(
        &self,
        snapshot: ClipboardSnapshot,
        sync_images: bool,
        app_handle: Option<AppHandle>,
    ) {
        let ClipboardSnapshot::Image {
            png_bytes,
            width,
            height,
        } = snapshot
        else {
            return;
        };

        if !sync_images {
            return;
        }

        let is_connected = *self.connected.lock().await;
        if !is_connected {
            notify_sync_failure(
                app_handle.as_ref(),
                SYNC_NOT_CONNECTED_MESSAGE,
            );
            return;
        }

        let size = png_bytes.len();
        if size > MAX_IMAGE_BYTES {
            notify_sync_failure(app_handle.as_ref(), "图片超过 5 MB，未同步到其他设备。");
            return;
        }

        if size <= INLINE_IMAGE_BYTES {
            self.send_image_inline(&png_bytes, width, height, app_handle.as_ref());
            return;
        }

        if !self.supports_chunked_images() {
            notify_sync_failure(
                app_handle.as_ref(),
                "图片较大，当前连接方式暂不支持同步超过 512 KB 的图片。",
            );
            return;
        }

        let hash = *blake3::hash(&png_bytes).as_bytes();
        let mut progress = TransferProgressReporter::image_send(app_handle.clone(), size as u64);

        let send_signal = |msg: SignalMessage| self.send_signal(msg);
        let send_binary = |chunk: BinaryChunk| self.send_binary(chunk);
        let progress_sink = |acked: u32, chunk_total: u32| {
            let force = acked == 0 || acked >= chunk_total;
            progress.report_chunks(acked, chunk_total, size as u64, force);
        };
        let result = send_image_with_flow_control(
            send_signal,
            send_binary,
            &self.ack_waiters,
            &self.transfer_cancel_reasons,
            &png_bytes,
            width,
            height,
            hash,
            Some(progress_sink),
        )
        .await;

        match result {
            Ok(()) => {
                progress.finish("图片已同步");
            }
            Err(error) => {
                tracing::warn!("Chunked image send failed: {error}");
                notify_sync_failure(app_handle.as_ref(), &error);
            }
        }
    }

    fn send_text(&self, text: &str) {
        let hash = *blake3::hash(text.as_bytes()).as_bytes();
        let hash_hex = hex::encode(hash);

        self.send_signal(SignalMessage::Clipboard {
            payload: text.to_string(),
            hash: hash_hex,
        });
    }

    fn send_image_inline(
        &self,
        png_bytes: &[u8],
        width: u32,
        height: u32,
        app_handle: Option<&AppHandle>,
    ) {
        let hash = *blake3::hash(&png_bytes).as_bytes();
        let hash_hex = hex::encode(hash);

        emit_sync_activity(app_handle, true, "image", "正在同步图片…");

        self.send_signal(SignalMessage::ClipboardImageInline {
            hash: hash_hex,
            width,
            height,
            mime: "image/png".to_string(),
            data_base64: BASE64.encode(png_bytes),
        });

        emit_sync_activity(app_handle, false, "image", "图片已同步");
    }
}

async fn finalize_received_file_names_as_text(
    text: String,
    dedup: &Arc<Mutex<DedupStore>>,
    clip_tx: &broadcast::Sender<ClipboardEvent>,
    peer_name: &str,
) {
    let content = text.trim().to_string();
    if content.is_empty() {
        return;
    }

    let hash_bytes = *blake3::hash(content.as_bytes()).as_bytes();
    {
        let mut d = dedup.lock().await;
        if d.has_seen(&hash_bytes) {
            return;
        }
        d.mark_seen(hash_bytes);
    }

    tracing::info!("Received remote clipboard file name(s) as text: {content}");
    ClipboardMonitor::write_clipboard(&content);
    let snapshot = ClipboardSnapshot::Text(content);
    let _ = clip_tx.send(ClipboardEvent::remote(snapshot, peer_name.to_string()));
}

async fn finalize_received_file(
    completed: CompletedFile,
    dedup: &Arc<Mutex<DedupStore>>,
    clip_tx: &broadcast::Sender<ClipboardEvent>,
    peer_name: &str,
    _app_handle: Option<&AppHandle>,
    pending_paths: Option<Vec<PathBuf>>,
) {
    let paths = pending_paths.unwrap_or_else(|| vec![completed.path.clone()]);
    let files = paths
        .iter()
        .filter_map(|path| {
            let metadata = std::fs::metadata(path).ok()?;
            let content_hash = crate::clipboard::file::hash_file(path).ok()?;
            Some(ClipboardFileItem {
                file_name: path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&completed.file_name)
                    .to_string(),
                size_bytes: metadata.len(),
                content_hash,
                source_path: Some(path.clone()),
            })
        })
        .collect::<Vec<_>>();

    if files.is_empty() {
        return;
    }

    let list_hash = file_list_hash(&files);
    {
        let mut d = dedup.lock().await;
        if d.has_seen(&list_hash) {
            return;
        }
        d.mark_seen(list_hash);
    }

    tracing::info!(
        "Received remote clipboard file(s): {} ({})",
        crate::clipboard::file::file_list_summary(&files),
        format_byte_size(files.iter().map(|file| file.size_bytes).sum::<u64>() as usize)
    );

    ClipboardMonitor::write_clipboard_files(&paths);

    let snapshot = ClipboardSnapshot::FileList { files };
    let _ = clip_tx.send(ClipboardEvent::remote(snapshot, peer_name.to_string()));
}

async fn finalize_received_image(
    completed: CompletedImage,
    dedup: &Arc<Mutex<DedupStore>>,
    clip_tx: &broadcast::Sender<ClipboardEvent>,
    peer_name: &str,
    _app_handle: Option<&AppHandle>,
) {
    {
        let mut d = dedup.lock().await;
        if d.has_seen(&completed.hash) {
            return;
        }
        d.mark_seen(completed.hash);
    }

    tracing::info!(
        "Received remote clipboard image: {}x{} ({})",
        completed.width,
        completed.height,
        format_byte_size(completed.png_bytes.len())
    );
    ClipboardMonitor::write_clipboard_image(
        &completed.png_bytes,
        completed.width,
        completed.height,
    );
    let snapshot = ClipboardSnapshot::Image {
        png_bytes: completed.png_bytes,
        width: completed.width,
        height: completed.height,
    };
    let _ = clip_tx.send(ClipboardEvent::remote(snapshot, peer_name.to_string()));
}

fn report_receive_transfer_progress(
    file_session: &FileReceiveSession,
    file_progress: &mut Option<TransferProgressReporter>,
    image_session: &ImageReceiveSession,
    image_progress: &mut Option<TransferProgressReporter>,
) {
    if let Some((received, chunk_total, _, file_name, _, bytes_done, bytes_total)) =
        file_session.receive_progress()
    {
        if let Some(reporter) = file_progress.as_mut() {
            reporter.set_label(file_name);
            let force = received == 0 || received >= chunk_total;
            reporter.report_bytes(bytes_done, bytes_total, force);
        }
    }

    if let Some((received, chunk_total, total_bytes)) = image_session.receive_progress() {
        if let Some(reporter) = image_progress.as_mut() {
            let bytes_done = if chunk_total > 0 {
                ((received as f64 / chunk_total as f64) * total_bytes as f64) as u64
            } else {
                0
            };
            let force = received == 0 || received >= chunk_total;
            reporter.report_bytes(bytes_done, total_bytes.max(1), force);
        }
    }
}

async fn handle_incoming_signal(
    msg: SignalMessage,
    dedup: &Arc<Mutex<DedupStore>>,
    clip_tx: &broadcast::Sender<ClipboardEvent>,
    peer_name: &str,
    app_handle: Option<&AppHandle>,
    connected: Option<&Arc<Mutex<bool>>>,
    stop_on_peer_left: bool,
    image_session: &mut ImageReceiveSession,
    file_session: &mut FileReceiveSession,
    handle: &ConnectionHandle,
    max_file_bytes: u64,
    file_progress: &mut Option<TransferProgressReporter>,
    image_progress: &mut Option<TransferProgressReporter>,
) -> bool {
    match msg {
        SignalMessage::PeerJoined { peer_id } => {
            tracing::info!("Peer joined room: {peer_id}");
            true
        }
        SignalMessage::PeerLeft { peer_id } => {
            tracing::info!("Peer left room: {peer_id}");
            if let Some(connected) = connected {
                *connected.lock().await = false;
            }
            !stop_on_peer_left
        }
        SignalMessage::TransferAck {
            transfer_id,
            chunk_index,
        } => {
            route_transfer_ack(&handle.ack_waiters, &transfer_id, chunk_index).await;
            true
        }
        SignalMessage::TransferCancel { transfer_id, reason } => {
            image_session.cancel(&transfer_id);
            file_session.cancel(&transfer_id);
            *file_progress = None;
            *image_progress = None;
            cancel_transfer_ack(
                &handle.ack_waiters,
                &handle.transfer_cancel_reasons,
                &transfer_id,
                reason,
            )
            .await;
            true
        }
        SignalMessage::ClipboardImageBegin {
            transfer_id,
            hash,
            width,
            height,
            total_bytes,
            chunk_size,
        } => {
            let hash_bytes = match decode_hash(&hash) {
                Some(value) => value,
                None => {
                    tracing::warn!("Invalid hash in image begin");
                    return true;
                }
            };

            if total_bytes as usize > MAX_IMAGE_BYTES {
                handle.send_signal(SignalMessage::TransferCancel {
                    transfer_id: transfer_id.clone(),
                    reason: Some(peer_image_too_large_message()),
                });
                notify_sync_failure(app_handle, "收到的图片过大，已忽略。");
                return true;
            }

            if image_session
                .begin(
                    transfer_id.clone(),
                    hash_bytes,
                    width,
                    height,
                    total_bytes,
                    chunk_size,
                )
                .is_ok()
            {
                *image_progress = app_handle.cloned().map(|app| {
                    TransferProgressReporter::image_receive(Some(app), total_bytes)
                });
            } else {
                handle.send_signal(SignalMessage::TransferCancel {
                    transfer_id,
                    reason: Some(peer_image_receive_declined_message()),
                });
                notify_sync_failure(app_handle, "无法接收这张图片。");
            }
            true
        }
        SignalMessage::ClipboardImageEnd { transfer_id, hash } => {
            let hash_bytes = match decode_hash(&hash) {
                Some(value) => value,
                None => return true,
            };

            if let Some(completed) = image_session.end(&transfer_id, hash_bytes) {
                if let Some(reporter) = image_progress.as_mut() {
                    reporter.finish("图片已同步");
                }
                *image_progress = None;
                finalize_received_image(completed, dedup, clip_tx, peer_name, app_handle).await;
            } else {
                *image_progress = None;
                notify_sync_failure(app_handle, "图片接收未完成，已忽略。");
            }
            true
        }
        SignalMessage::ClipboardFileBegin {
            transfer_id,
            hash,
            file_name,
            total_bytes,
            chunk_size,
            batch_id,
            batch_index,
            batch_total,
        } => {
            let hash_bytes = match decode_hash(&hash) {
                Some(value) => value,
                None => {
                    tracing::warn!("Invalid hash in file begin");
                    return true;
                }
            };

            let receiver_sync_files = read_sync_files_enabled(app_handle).await;
            if !receiver_sync_files {
                handle.send_signal(SignalMessage::TransferCancel {
                    transfer_id: transfer_id.clone(),
                    reason: Some(PEER_TRANSFER_HANDLED.to_string()),
                });
                finalize_received_file_names_as_text(
                    file_name.clone(),
                    dedup,
                    clip_tx,
                    peer_name,
                )
                .await;
                return true;
            }

            if total_bytes > max_file_bytes {
                handle.send_signal(SignalMessage::TransferCancel {
                    transfer_id: transfer_id.clone(),
                    reason: Some(peer_file_too_large_message(max_file_bytes_to_mb(
                        max_file_bytes,
                    ))),
                });
                notify_sync_failure(
                    app_handle,
                    &format!(
                        "收到的文件 {} 超过本机 {} 的上限，已忽略。可在设置中调大「文件大小上限」。",
                        file_name,
                        format_byte_size(max_file_bytes as usize)
                    ),
                );
                return true;
            }

            if let (Some(batch_id), Some(batch_total)) = (batch_id.clone(), batch_total) {
                if !file_session.has_batch(&batch_id) {
                    file_session.register_batch(batch_id, batch_total);
                }
            }

            match file_session.begin(
                transfer_id.clone(),
                hash_bytes,
                file_name.clone(),
                total_bytes,
                chunk_size,
                batch_id.clone(),
                max_file_bytes,
            ) {
                Ok(()) => {
                    *file_progress = app_handle.cloned().map(|app| {
                        TransferProgressReporter::file_receive(
                            Some(app),
                            file_name.clone(),
                            batch_index,
                            batch_total,
                            0,
                            0,
                        )
                    });
                }
                Err(reason) => {
                    tracing::warn!("File receive begin failed for {file_name}: {reason}");
                    handle.send_signal(SignalMessage::TransferCancel {
                        transfer_id,
                        reason: Some(peer_file_receive_declined_message()),
                    });
                    notify_sync_failure(app_handle, "无法接收这个文件。");
                }
            }
            true
        }
        SignalMessage::ClipboardFileEnd { transfer_id, hash } => {
            let hash_bytes = match decode_hash(&hash) {
                Some(value) => value,
                None => return true,
            };

            if let Some(completed) = file_session.end(&transfer_id, hash_bytes) {
                let file_bytes = std::fs::metadata(&completed.path)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                if let Some(batch_id) = completed.batch_id.clone() {
                    file_session.mark_file_completed_in_batch(Some(&batch_id), file_bytes);
                    file_session.push_batch_file(&batch_id, completed.path.clone());
                } else {
                    if let Some(reporter) = file_progress.as_mut() {
                        reporter.finish("文件已同步");
                    }
                    *file_progress = None;
                    finalize_received_file(
                        completed,
                        dedup,
                        clip_tx,
                        peer_name,
                        app_handle,
                        None,
                    )
                    .await;
                }
            } else {
                *file_progress = None;
                notify_sync_failure(app_handle, "文件接收未完成，已忽略。");
            }
            true
        }
        SignalMessage::ClipboardFileBatchEnd { batch_id, file_count } => {
            if let Some(paths) = file_session.finalize_batch(&batch_id, file_count) {
                if let Some(first_path) = paths.first() {
                    let file_name = first_path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("file")
                        .to_string();
                    let batch_hash = *blake3::hash(batch_id.as_bytes()).as_bytes();
                    finalize_received_file(
                        CompletedFile {
                            path: first_path.clone(),
                            file_name,
                            hash: batch_hash,
                            batch_id: Some(batch_id),
                        },
                        dedup,
                        clip_tx,
                        peer_name,
                        app_handle,
                        Some(paths),
                    )
                    .await;
                }
                if let Some(reporter) = file_progress.as_mut() {
                    reporter.finish("文件已同步");
                }
                *file_progress = None;
            } else {
                *file_progress = None;
                notify_sync_failure(app_handle, "文件批次接收未完成，已忽略。");
            }
            true
        }
        SignalMessage::ClipboardFileListMeta { hash: _, files } => {
            let parsed_files = files
                .into_iter()
                .map(|file| ClipboardFileItem {
                    file_name: file.file_name,
                    size_bytes: file.size_bytes,
                    content_hash: decode_hash(&file.content_hash).unwrap_or([0u8; 32]),
                    source_path: None,
                })
                .collect::<Vec<_>>();

            if parsed_files.is_empty() {
                return true;
            }

            finalize_received_file_names_as_text(
                file_list_summary(&parsed_files),
                dedup,
                clip_tx,
                peer_name,
            )
            .await;
            true
        }
        SignalMessage::Clipboard { payload, hash } => {
            let hash_bytes = match decode_hash(&hash) {
                Some(value) => value,
                None => {
                    tracing::warn!("Invalid hash in clipboard message");
                    return true;
                }
            };

            {
                let mut d = dedup.lock().await;
                if d.has_seen(&hash_bytes) {
                    return true;
                }
                d.mark_seen(hash_bytes);
            }

            tracing::info!("Received remote clipboard: {} chars", payload.len());
            ClipboardMonitor::write_clipboard(&payload);
            let snapshot = ClipboardSnapshot::Text(payload);
            let _ = clip_tx.send(ClipboardEvent::remote(snapshot, peer_name.to_string()));
            true
        }
        SignalMessage::ClipboardImageInline {
            hash,
            width,
            height,
            mime,
            data_base64,
        } => {
            if mime != "image/png" {
                tracing::warn!("Unsupported remote image mime: {mime}");
                return true;
            }

            let hash_bytes = match decode_hash(&hash) {
                Some(value) => value,
                None => {
                    tracing::warn!("Invalid hash in image message");
                    return true;
                }
            };

            let png_bytes = match BASE64.decode(data_base64.as_bytes()) {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!("Invalid image payload: {error}");
                    notify_sync_failure(app_handle, "图片同步失败，请重新复制图片后再试。");
                    return true;
                }
            };

            if png_bytes.len() > MAX_IMAGE_BYTES {
                notify_sync_failure(app_handle, "收到的图片过大，已忽略。");
                return true;
            }

            let payload_hash = *blake3::hash(&png_bytes).as_bytes();
            if payload_hash != hash_bytes {
                tracing::warn!("Image hash mismatch");
                notify_sync_failure(app_handle, "图片同步失败，请重新复制图片后再试。");
                return true;
            }

            finalize_received_image(
                CompletedImage {
                    png_bytes,
                    width,
                    height,
                    hash: hash_bytes,
                },
                dedup,
                clip_tx,
                peer_name,
                app_handle,
            )
            .await;
            true
        }
    }
}

fn decode_hash(hash: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(hash).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

async fn read_sync_files_enabled(app_handle: Option<&AppHandle>) -> bool {
    let Some(app_handle) = app_handle else {
        return true;
    };
    let state = app_handle.state::<crate::AppState>();
    let sync_files = state.config.lock().await.sync_files.unwrap_or(true);
    sync_files
}

fn new_handle(
    transport: HandleTransport,
    connected: Arc<Mutex<bool>>,
    dedup: Arc<Mutex<DedupStore>>,
) -> ConnectionHandle {
    ConnectionHandle {
        transport,
        connected,
        dedup,
        ack_waiters: Arc::new(Mutex::new(HashMap::new())),
        transfer_cancel_reasons: Arc::new(Mutex::new(HashMap::new())),
    }
}

pub struct ConnectionManager;

impl ConnectionManager {
    pub async fn connect(
        server_url: &str,
        room: &str,
        peer_id: &str,
        connected: Arc<Mutex<bool>>,
        clip_tx: broadcast::Sender<ClipboardEvent>,
    ) -> Result<ConnectionHandle, Box<dyn std::error::Error>> {
        let client = signalling::connect(server_url, room, peer_id).await?;

        let sig_tx = client.tx.clone();
        let mut sig_rx = client.rx;
        let dedup = Arc::new(Mutex::new(DedupStore::new(128)));

        let handle = new_handle(
            HandleTransport::Signalling { tx: sig_tx },
            connected.clone(),
            dedup.clone(),
        );

        *connected.lock().await = true;

        let receive_handle = handle.clone();
        tokio::spawn(async move {
            let mut image_session = ImageReceiveSession::new();
            let mut file_session = FileReceiveSession::new();
            let mut file_progress = None;
            let mut image_progress = None;
            let max_file_bytes = crate::clipboard::file::DEFAULT_MAX_FILE_BYTES;
            while let Some(msg) = sig_rx.recv().await {
                if !handle_incoming_signal(
                    msg,
                    &dedup,
                    &clip_tx,
                    "已配对设备",
                    None,
                    Some(&connected),
                    false,
                    &mut image_session,
                    &mut file_session,
                    &receive_handle,
                    max_file_bytes,
                    &mut file_progress,
                    &mut image_progress,
                )
                .await
                {
                    break;
                }
            }

            tracing::warn!("Signalling connection lost");
            *connected.lock().await = false;
        });

        Ok(handle)
    }

    pub async fn connect_direct(
        conn: DirectConnection,
        connections: Arc<Mutex<ConnectionRegistry>>,
        connection_generation: Arc<AtomicU64>,
        session_generation: u64,
        clip_tx: broadcast::Sender<ClipboardEvent>,
        app_handle: AppHandle,
    ) -> ConnectionHandle {
        let peer_name = conn.peer_name.clone();
        let peer_id = conn.peer_id.clone();
        let event_tx = conn.tx;
        let mut event_rx = conn.rx;
        let dedup = Arc::new(Mutex::new(DedupStore::new(128)));
        let session_connected = Arc::new(Mutex::new(true));

        let handle = new_handle(
            HandleTransport::Direct { tx: event_tx },
            session_connected.clone(),
            dedup.clone(),
        );

        connections.lock().await.insert(
            peer_id.clone(),
            ConnectionSession {
                handle: handle.clone(),
                peer_name: peer_name.clone(),
                session_generation,
                connected: session_connected.clone(),
            },
        );

        let max_file_bytes = {
            let state = app_handle.state::<crate::AppState>();
            let config = state.config.lock().await;
            config
                .max_file_bytes
                .unwrap_or(crate::clipboard::file::DEFAULT_MAX_FILE_BYTES)
        };

        let receive_handle = handle.clone();
        tokio::spawn(async move {
            let mut peer_left = false;
            let mut image_session = ImageReceiveSession::new();
            let mut file_session = FileReceiveSession::new();
            let mut file_progress = None;
            let mut image_progress = None;

            while let Some(event) = event_rx.recv().await {
                let should_continue = match event {
                    ConnectionEvent::Signal(msg) => {
                        handle_incoming_signal(
                            msg,
                            &dedup,
                            &clip_tx,
                            &peer_name,
                            Some(&app_handle),
                            None,
                            true,
                            &mut image_session,
                            &mut file_session,
                            &receive_handle,
                            max_file_bytes,
                            &mut file_progress,
                            &mut image_progress,
                        )
                        .await
                    }
                    ConnectionEvent::Binary(chunk) => {
                        let send_ack = |msg: SignalMessage| receive_handle.send_signal(msg);
                        file_session.ingest_chunk(&chunk, &send_ack);
                        image_session.ingest_chunk(&chunk, &send_ack);
                        report_receive_transfer_progress(
                            &file_session,
                            &mut file_progress,
                            &image_session,
                            &mut image_progress,
                        );
                        true
                    }
                };

                if !should_continue {
                    peer_left = true;
                    break;
                }
            }

            tracing::warn!("Direct connection lost for peer {peer_id}");

            if connection_generation.load(Ordering::SeqCst) != session_generation {
                return;
            }

            let was_connected = {
                let mut registry = connections.lock().await;
                let removed = registry
                    .get(&peer_id)
                    .filter(|session| session.session_generation == session_generation)
                    .is_some();
                if removed {
                    registry.remove(&peer_id);
                }
                removed && *session_connected.lock().await
            };

            *session_connected.lock().await = false;

            if !was_connected {
                return;
            }

            {
                let state = app_handle.state::<crate::AppState>();
                crate::spawn_lan_presence_refresh(state.inner(), &app_handle);
            }

            let tcp_port = {
                let state = app_handle.state::<crate::AppState>();
                let port = state
                    .config
                    .lock()
                    .await
                    .tcp_port
                    .unwrap_or(crate::app_profile::DEFAULT_TCP_PORT);
                port
            };

            let (message, kind) = {
                let state = app_handle.state::<crate::AppState>();
                crate::resolve_connection_ended_message(
                    state.inner(),
                    &peer_id,
                    &peer_name,
                    peer_left,
                    tcp_port,
                )
                .await
            };

            if kind == "peer_offline" {
                let state = app_handle.state::<crate::AppState>();
                crate::remove_lan_device_by_peer_id(
                    &state.lan_devices,
                    &peer_id,
                    &app_handle,
                )
                .await;
            }

            let _ = app_handle.emit(
                "connection-ended",
                serde_json::json!({
                    "kind": kind,
                    "message": message,
                    "peer_name": peer_name,
                    "peer_id": peer_id,
                }),
            );
            crate::window::send_session_ended_notification(&app_handle, &message);
        });

        handle
    }
}
