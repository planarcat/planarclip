use std::collections::{HashMap, HashSet};
use std::time::Duration;

use tokio::sync::mpsc;

use crate::clipboard::image::MAX_IMAGE_BYTES;
use crate::network::binary_chunk::{transfer_id_to_bytes, BinaryChunk};
use crate::network::protocol::SignalMessage;

pub const CHUNK_SIZE: usize = 256 * 1024;
pub const SEND_WINDOW: u32 = 8;
const ACK_TIMEOUT: Duration = Duration::from_secs(15);

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
                Ok(None) => return Err("ack channel closed"),
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

pub async fn route_transfer_ack(
    registry: &std::sync::Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<u32>>>>,
    transfer_id: &str,
    chunk_index: u32,
) {
    if let Some(tx) = registry.lock().await.get(transfer_id) {
        let _ = tx.send(chunk_index);
    }
}
