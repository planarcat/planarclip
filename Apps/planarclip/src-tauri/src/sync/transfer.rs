use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::sync::mpsc;

use crate::clipboard::image::MAX_IMAGE_BYTES;
use crate::network::binary_chunk::{transfer_id_to_bytes, BinaryChunk};
use crate::network::protocol::SignalMessage;
use crate::storage::staging;

pub const CHUNK_SIZE: usize = 256 * 1024;
pub const SEND_WINDOW: u32 = 8;
const ACK_TIMEOUT: Duration = Duration::from_secs(15);

/// Opaque peer response: transfer was handled on the remote side (not an error for sender).
pub const PEER_TRANSFER_HANDLED: &str = "peer:handled";
/// Opaque peer response: transfer declined without exposing local settings.
pub const PEER_TRANSFER_DECLINED: &str = "peer:declined";
const PEER_FILE_TOO_LARGE_PREFIX: &str = "peer:file-too-large:";

pub fn max_file_bytes_to_mb(bytes: u64) -> u32 {
    ((bytes + 1024 * 1024 - 1) / (1024 * 1024)).max(1) as u32
}

pub fn peer_file_too_large_reason(limit_mb: u32) -> String {
    format!("{PEER_FILE_TOO_LARGE_PREFIX}{limit_mb}")
}

/// Map an opaque peer cancel reason to sender outcome. `Ok(())` means treat as success.
pub fn resolve_peer_transfer_cancel(reason: Option<String>) -> Result<(), String> {
    match reason {
        Some(ref reason) if reason == PEER_TRANSFER_HANDLED => Ok(()),
        Some(ref reason) if reason.starts_with(PEER_FILE_TOO_LARGE_PREFIX) => {
            let limit_mb = &reason[PEER_FILE_TOO_LARGE_PREFIX.len()..];
            Err(format!(
                "文件同步失败，对方回应：文件大小超出限制大小 {} MB。",
                limit_mb
            ))
        }
        Some(_) => Err("文件同步失败，对方拒绝了这次传输。".to_string()),
        None => Err("文件同步失败，请重新复制文件后再试。".to_string()),
    }
}

pub async fn take_peer_transfer_cancel(
    cancel_reasons: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    transfer_id: &str,
) -> Result<(), String> {
    let reason = cancel_reasons.lock().await.remove(transfer_id);
    resolve_peer_transfer_cancel(reason)
}

pub struct CompletedImage {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub hash: [u8; 32],
}

struct PendingImageReceive {
    hash: [u8; 32],
    width: u32,
    height: u32,
    buffer: Vec<u8>,
    chunk_size: u32,
    chunk_total: u32,
    received: HashSet<u32>,
}

pub struct ImageReceiveSession {
    active: Option<(String, PendingImageReceive)>,
}

impl ImageReceiveSession {
    pub fn new() -> Self {
        Self { active: None }
    }

    pub fn begin(
        &mut self,
        transfer_id: String,
        hash: [u8; 32],
        width: u32,
        height: u32,
        total_bytes: u64,
        chunk_size: u32,
    ) -> Result<(), &'static str> {
        if total_bytes as usize > MAX_IMAGE_BYTES {
            return Err("image too large");
        }

        let chunk_total = chunk_total_for_size(total_bytes as usize, chunk_size as usize);
        let buffer = vec![0u8; total_bytes as usize];
        self.active = Some((
            transfer_id,
            PendingImageReceive {
                hash,
                width,
                height,
                buffer,
                chunk_size,
                chunk_total,
                received: HashSet::new(),
            },
        ));
        Ok(())
    }

    pub fn ingest_chunk(
        &mut self,
        chunk: &BinaryChunk,
        send_ack: &impl Fn(SignalMessage),
    ) {
        let Some((transfer_id, pending)) = self.active.as_mut() else {
            return;
        };
        let Some(expected_id) = transfer_id_to_bytes(transfer_id) else {
            return;
        };
        if chunk.transfer_id != expected_id {
            return;
        }
        if chunk.chunk_total != pending.chunk_total {
            return;
        }
        if chunk.chunk_index >= pending.chunk_total {
            return;
        }

        let chunk_size = pending.chunk_size.max(1) as usize;
        let offset = chunk.chunk_index as usize * chunk_size;
        if offset + chunk.payload.len() > pending.buffer.len() {
            return;
        }

        pending.buffer[offset..offset + chunk.payload.len()].copy_from_slice(&chunk.payload);
        pending.received.insert(chunk.chunk_index);

        send_ack(SignalMessage::TransferAck {
            transfer_id: transfer_id.clone(),
            chunk_index: chunk.chunk_index,
        });
    }

    pub fn end(&mut self, transfer_id: &str, hash: [u8; 32]) -> Option<CompletedImage> {
        let (active_id, pending) = self.active.as_ref()?;
        if active_id != transfer_id {
            return None;
        }
        if pending.hash != hash {
            tracing::warn!("Image end hash mismatch");
            self.active = None;
            return None;
        }
        if pending.received.len() as u32 != pending.chunk_total {
            tracing::warn!("Image transfer incomplete at end");
            self.active = None;
            return None;
        }
        self.finalize_active()
    }

    pub fn cancel(&mut self, transfer_id: &str) {
        if self
            .active
            .as_ref()
            .is_some_and(|(active_id, _)| active_id == transfer_id)
        {
            self.active = None;
        }
    }

    fn finalize_active(&mut self) -> Option<CompletedImage> {
        let (_, pending) = self.active.take()?;
        let actual_hash = *blake3::hash(&pending.buffer).as_bytes();
        if actual_hash != pending.hash {
            tracing::warn!("Assembled image hash mismatch");
            return None;
        }

        Some(CompletedImage {
            png_bytes: pending.buffer,
            width: pending.width,
            height: pending.height,
            hash: pending.hash,
        })
    }
}

pub fn chunk_total_for_size(total_bytes: usize, chunk_size: usize) -> u32 {
    let chunk_size = chunk_size.max(1);
    ((total_bytes + chunk_size - 1) / chunk_size) as u32
}

pub struct AckWaiter {
    rx: mpsc::UnboundedReceiver<u32>,
}

impl AckWaiter {
    pub async fn register(
        registry: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<u32>>>>,
        transfer_id: &str,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        registry
            .lock()
            .await
            .insert(transfer_id.to_string(), tx);
        Self { rx }
    }

    pub async fn unregister(
        registry: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<u32>>>>,
        transfer_id: &str,
    ) {
        registry.lock().await.remove(transfer_id);
    }

    pub async fn wait_any(&mut self) -> Option<u32> {
        self.rx.recv().await
    }
}

pub fn build_binary_chunk(
    transfer_id: &str,
    chunk_index: u32,
    chunk_total: u32,
    png_bytes: &[u8],
) -> Option<BinaryChunk> {
    let transfer_bytes = transfer_id_to_bytes(transfer_id)?;
    let start = chunk_index as usize * CHUNK_SIZE;
    if start >= png_bytes.len() {
        return None;
    }
    let end = (start + CHUNK_SIZE).min(png_bytes.len());
    Some(BinaryChunk {
        transfer_id: transfer_bytes,
        chunk_index,
        chunk_total,
        payload: png_bytes[start..end].to_vec(),
    })
}

pub async fn send_image_with_flow_control(
    send_signal: impl Fn(SignalMessage),
    send_binary: impl Fn(BinaryChunk),
    ack_registry: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<u32>>>>,
    _cancel_reasons: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    png_bytes: &[u8],
    width: u32,
    height: u32,
    hash: [u8; 32],
) -> Result<(), &'static str> {
    let transfer_id = uuid::Uuid::new_v4().to_string();
    let chunk_total = chunk_total_for_size(png_bytes.len(), CHUNK_SIZE);
    let hash_hex = hex::encode(hash);

    send_signal(SignalMessage::ClipboardImageBegin {
        transfer_id: transfer_id.clone(),
        hash: hash_hex.clone(),
        width,
        height,
        total_bytes: png_bytes.len() as u64,
        chunk_size: CHUNK_SIZE as u32,
    });

    let mut ack_waiter = AckWaiter::register(ack_registry, &transfer_id).await;
    let mut next_index = 0u32;
    let mut in_flight = 0u32;
    let mut acked = HashSet::new();

    let transfer_result = async {
        while next_index < chunk_total || in_flight > 0 {
            while in_flight < SEND_WINDOW && next_index < chunk_total {
                let chunk = build_binary_chunk(&transfer_id, next_index, chunk_total, png_bytes)
                    .ok_or("chunk build failed")?;
                send_binary(chunk);
                next_index += 1;
                in_flight += 1;
            }

            if in_flight == 0 {
                break;
            }

            match tokio::time::timeout(ACK_TIMEOUT, ack_waiter.wait_any()).await {
                Ok(Some(chunk_index)) => {
                    if !acked.insert(chunk_index) {
                        continue;
                    }
                    in_flight = in_flight.saturating_sub(1);
                }
                Ok(None) => return Err("transfer cancelled"),
                Err(_) => return Err("ack timeout"),
            }
        }

        send_signal(SignalMessage::ClipboardImageEnd {
            transfer_id: transfer_id.clone(),
            hash: hash_hex,
        });

        Ok(())
    }
    .await;

    AckWaiter::unregister(ack_registry, &transfer_id).await;

    if transfer_result.is_err() {
        send_signal(SignalMessage::TransferCancel {
            transfer_id,
            reason: Some("send_failed".into()),
        });
    }

    transfer_result
}

pub struct CompletedFile {
    pub path: PathBuf,
    pub file_name: String,
    pub hash: [u8; 32],
    pub batch_id: Option<String>,
}

struct PendingFileReceive {
    hash: [u8; 32],
    file_name: String,
    batch_id: Option<String>,
    temp_path: PathBuf,
    total_bytes: u64,
    chunk_size: u32,
    chunk_total: u32,
    received: HashSet<u32>,
}

struct PendingFileBatch {
    batch_id: String,
    paths: Vec<PathBuf>,
    expected_count: u32,
}

pub struct FileReceiveSession {
    active: Option<(String, PendingFileReceive)>,
    batch: Option<PendingFileBatch>,
}

impl FileReceiveSession {
    pub fn new() -> Self {
        Self {
            active: None,
            batch: None,
        }
    }

    pub fn begin(
        &mut self,
        transfer_id: String,
        hash: [u8; 32],
        file_name: String,
        total_bytes: u64,
        chunk_size: u32,
        batch_id: Option<String>,
        max_file_bytes: u64,
    ) -> Result<(), &'static str> {
        if total_bytes > max_file_bytes {
            return Err("file too large");
        }

        staging::ensure_staging().map_err(|_| "staging unavailable")?;
        let temp_path = staging::temp_transfer_path(&transfer_id);
        let _ = fs::remove_file(&temp_path);
        File::create(&temp_path).map_err(|_| "temp file create failed")?;

        let chunk_total = chunk_total_for_size(total_bytes as usize, chunk_size as usize);
        self.active = Some((
            transfer_id,
            PendingFileReceive {
                hash,
                file_name,
                batch_id,
                temp_path,
                total_bytes,
                chunk_size,
                chunk_total,
                received: HashSet::new(),
            },
        ));
        Ok(())
    }

    pub fn ingest_chunk(
        &mut self,
        chunk: &BinaryChunk,
        send_ack: &impl Fn(SignalMessage),
    ) {
        let Some((transfer_id, pending)) = self.active.as_mut() else {
            return;
        };
        let Some(expected_id) = transfer_id_to_bytes(transfer_id) else {
            return;
        };
        if chunk.transfer_id != expected_id {
            return;
        }
        if chunk.chunk_total != pending.chunk_total {
            return;
        }
        if chunk.chunk_index >= pending.chunk_total {
            return;
        }

        let chunk_size = pending.chunk_size.max(1) as usize;
        let offset = chunk.chunk_index as usize * chunk_size;
        if offset + chunk.payload.len() > pending.total_bytes as usize {
            return;
        }

        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .open(&pending.temp_path)
        {
            if file.seek(SeekFrom::Start(offset as u64)).is_ok() {
                let _ = file.write_all(&chunk.payload);
            }
        }

        pending.received.insert(chunk.chunk_index);
        send_ack(SignalMessage::TransferAck {
            transfer_id: transfer_id.clone(),
            chunk_index: chunk.chunk_index,
        });
    }

    pub fn end(&mut self, transfer_id: &str, hash: [u8; 32]) -> Option<CompletedFile> {
        let (active_id, pending) = self.active.as_ref()?;
        if active_id != transfer_id {
            return None;
        }
        if pending.hash != hash {
            tracing::warn!("File end hash mismatch");
            self.active = None;
            return None;
        }
        if pending.received.len() as u32 != pending.chunk_total {
            tracing::warn!("File transfer incomplete at end");
            let temp_path = pending.temp_path.clone();
            self.active = None;
            let _ = fs::remove_file(temp_path);
            return None;
        }

        let (_, pending) = self.active.take()?;
        let actual_hash = hash_file_path(&pending.temp_path).ok()?;
        if actual_hash != pending.hash {
            tracing::warn!("Assembled file hash mismatch");
            let _ = fs::remove_file(&pending.temp_path);
            return None;
        }

        let final_path = staging::finalize_staged_file(
            &pending.temp_path,
            pending.batch_id.as_deref(),
            &pending.file_name,
        )
        .ok()?;

        Some(CompletedFile {
            path: final_path,
            file_name: pending.file_name,
            hash: pending.hash,
            batch_id: pending.batch_id,
        })
    }

    pub fn cancel(&mut self, transfer_id: &str) {
        if let Some((active_id, pending)) = self.active.take() {
            if active_id == transfer_id {
                let _ = fs::remove_file(&pending.temp_path);
            } else {
                self.active = Some((active_id, pending));
            }
        }
    }

    pub fn register_batch(&mut self, batch_id: String, expected_count: u32) {
        self.batch = Some(PendingFileBatch {
            batch_id,
            paths: Vec::new(),
            expected_count,
        });
    }

    pub fn has_batch(&self, batch_id: &str) -> bool {
        self.batch
            .as_ref()
            .is_some_and(|batch| batch.batch_id == batch_id)
    }

    pub fn push_batch_file(&mut self, batch_id: &str, path: PathBuf) -> bool {
        let Some(batch) = self.batch.as_mut() else {
            return false;
        };
        if batch.batch_id != batch_id {
            return false;
        }
        batch.paths.push(path);
        true
    }

    pub fn finalize_batch(&mut self, batch_id: &str, file_count: u32) -> Option<Vec<PathBuf>> {
        let batch = self.batch.take()?;
        if batch.batch_id != batch_id || batch.expected_count != file_count {
            return None;
        }
        if batch.paths.len() as u32 != file_count {
            return None;
        }
        Some(batch.paths)
    }
}

pub async fn send_file_with_flow_control(
    send_signal: impl Fn(SignalMessage),
    send_binary: impl Fn(BinaryChunk),
    ack_registry: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<u32>>>>,
    cancel_reasons: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    file_path: &Path,
    file_name: &str,
    hash: [u8; 32],
    batch_id: Option<String>,
    batch_index: Option<u32>,
    batch_total: Option<u32>,
) -> Result<(), String> {
    let metadata = fs::metadata(file_path).map_err(|_| "file metadata failed".to_string())?;
    let total_bytes = metadata.len();
    let transfer_id = uuid::Uuid::new_v4().to_string();
    let chunk_total = chunk_total_for_size(total_bytes as usize, CHUNK_SIZE);
    let hash_hex = hex::encode(hash);

    send_signal(SignalMessage::ClipboardFileBegin {
        transfer_id: transfer_id.clone(),
        hash: hash_hex.clone(),
        file_name: file_name.to_string(),
        total_bytes,
        chunk_size: CHUNK_SIZE as u32,
        batch_id,
        batch_index,
        batch_total,
    });

    let mut ack_waiter = AckWaiter::register(ack_registry, &transfer_id).await;
    let mut next_index = 0u32;
    let mut in_flight = 0u32;
    let mut acked = HashSet::new();
    let mut file = File::open(file_path).map_err(|_| "file open failed".to_string())?;

    let transfer_result = async {
        while next_index < chunk_total || in_flight > 0 {
            while in_flight < SEND_WINDOW && next_index < chunk_total {
                let chunk = build_file_binary_chunk(&transfer_id, next_index, chunk_total, &mut file)
                    .ok_or_else(|| "chunk build failed".to_string())?;
                send_binary(chunk);
                next_index += 1;
                in_flight += 1;
            }

            if in_flight == 0 {
                break;
            }

            match tokio::time::timeout(ACK_TIMEOUT, ack_waiter.wait_any()).await {
                Ok(Some(chunk_index)) => {
                    if !acked.insert(chunk_index) {
                        continue;
                    }
                    in_flight = in_flight.saturating_sub(1);
                }
                Ok(None) => {
                    return take_peer_transfer_cancel(cancel_reasons, &transfer_id).await;
                }
                Err(_) => return Err("ack timeout".to_string()),
            }
        }

        send_signal(SignalMessage::ClipboardFileEnd {
            transfer_id: transfer_id.clone(),
            hash: hash_hex,
        });

        Ok(())
    }
    .await;

    AckWaiter::unregister(ack_registry, &transfer_id).await;

    if transfer_result.is_err() {
        send_signal(SignalMessage::TransferCancel {
            transfer_id,
            reason: Some("send_failed".into()),
        });
    }

    transfer_result.map_err(map_file_send_error)
}

fn map_file_send_error(error: String) -> String {
    match error.as_str() {
        "ack timeout" => "文件同步超时，请检查网络连接后重新复制文件。".to_string(),
        "file metadata failed" | "file open failed" | "chunk build failed" => {
            "文件同步失败，请重新复制文件后再试。".to_string()
        }
        other => other.to_string(),
    }
}

pub fn build_file_binary_chunk(
    transfer_id: &str,
    chunk_index: u32,
    chunk_total: u32,
    file: &mut File,
) -> Option<BinaryChunk> {
    let transfer_bytes = transfer_id_to_bytes(transfer_id)?;
    let start = chunk_index as u64 * CHUNK_SIZE as u64;
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut payload = vec![0u8; CHUNK_SIZE];
    let read = file.read(&mut payload).ok()?;
    if read == 0 {
        return None;
    }
    payload.truncate(read);
    Some(BinaryChunk {
        transfer_id: transfer_bytes,
        chunk_index,
        chunk_total,
        payload,
    })
}

fn hash_file_path(path: &Path) -> Result<[u8; 32], String> {
    crate::clipboard::file::hash_file(path)
}

pub async fn route_transfer_ack(
    registry: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<u32>>>>,
    transfer_id: &str,
    chunk_index: u32,
) {
    if let Some(tx) = registry.lock().await.get(transfer_id) {
        let _ = tx.send(chunk_index);
    }
}

/// Drop the sender-side ack waiter so in-flight transfers fail fast instead of timing out.
pub async fn cancel_transfer_ack(
    registry: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<u32>>>>,
    cancel_reasons: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    transfer_id: &str,
    reason: Option<String>,
) {
    if let Some(reason) = reason {
        cancel_reasons
            .lock()
            .await
            .insert(transfer_id.to_string(), reason);
    }
    registry.lock().await.remove(transfer_id);
}
