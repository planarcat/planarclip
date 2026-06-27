use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::clipboard::image::{format_byte_size, INLINE_IMAGE_BYTES, MAX_IMAGE_BYTES};
use crate::clipboard::monitor::ClipboardMonitor;
use crate::clipboard::types::{ClipboardEvent, ClipboardSnapshot};
use crate::network::binary_chunk::BinaryChunk;
use crate::network::direct::{ConnectionEvent, DirectConnection};
use crate::network::protocol::SignalMessage;
use crate::network::signalling;
use crate::sync::dedup::DedupStore;
use crate::sync::transfer::{
    route_transfer_ack, send_image_with_flow_control, CompletedImage, ImageReceiveSession,
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
}

impl ConnectionHandle {
    pub fn connected(&self) -> Arc<Mutex<bool>> {
        self.connected.clone()
    }

    pub fn supports_chunked_images(&self) -> bool {
        matches!(self.transport, HandleTransport::Direct { .. })
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
                    emit_sync_notice(app_handle, "图片超过 5 MB，未同步到其他设备。");
                    return;
                }

                if size > INLINE_IMAGE_BYTES {
                    if !self.supports_chunked_images() {
                        emit_sync_notice(
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
            return;
        }

        let size = png_bytes.len();
        if size > MAX_IMAGE_BYTES {
            emit_sync_notice(app_handle.as_ref(), "图片超过 5 MB，未同步到其他设备。");
            return;
        }

        if size <= INLINE_IMAGE_BYTES {
            self.send_image_inline(&png_bytes, width, height, app_handle.as_ref());
            return;
        }

        if !self.supports_chunked_images() {
            emit_sync_notice(
                app_handle.as_ref(),
                "图片较大，当前连接方式暂不支持同步超过 512 KB 的图片。",
            );
            return;
        }

        let hash = *blake3::hash(&png_bytes).as_bytes();

        emit_sync_activity(app_handle.as_ref(), true, "image", "正在同步图片…");

        let send_signal = |msg: SignalMessage| self.send_signal(msg);
        let send_binary = |chunk: BinaryChunk| self.send_binary(chunk);
        let result = send_image_with_flow_control(
            send_signal,
            send_binary,
            &self.ack_waiters,
            &png_bytes,
            width,
            height,
            hash,
        )
        .await;

        match result {
            Ok(()) => {
                emit_sync_activity(app_handle.as_ref(), false, "image", "图片已同步");
            }
            Err(error) => {
                tracing::warn!("Chunked image send failed: {error}");
                let message = match error {
                    "ack timeout" => {
                        "图片同步超时，请确认对方设备已更新到最新版本，然后重新复制图片。"
                    }
                    "ack channel closed" => "图片同步失败，与对方设备的连接已中断。",
                    _ => "图片同步失败，请稍后再试。",
                };
                emit_sync_notice(app_handle.as_ref(), message);
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

fn emit_sync_activity(app_handle: Option<&AppHandle>, active: bool, kind: &str, message: &str) {
    let Some(app_handle) = app_handle else {
        return;
    };
    let _ = app_handle.emit(
        "clipboard-sync-activity",
        serde_json::json!({
            "active": active,
            "kind": kind,
            "message": message,
        }),
    );
}

fn emit_sync_notice(app_handle: Option<&AppHandle>, message: &str) {
    let Some(app_handle) = app_handle else {
        return;
    };
    let _ = app_handle.emit(
        "clipboard-sync-activity",
        serde_json::json!({
            "active": false,
            "kind": "notice",
            "message": message,
        }),
    );
}

async fn finalize_received_image(
    completed: CompletedImage,
    dedup: &Arc<Mutex<DedupStore>>,
    clip_tx: &broadcast::Sender<ClipboardEvent>,
    peer_name: &str,
    app_handle: Option<&AppHandle>,
) {
    {
        let mut d = dedup.lock().await;
        if d.has_seen(&completed.hash) {
            return;
        }
        d.mark_seen(completed.hash);
    }

    emit_sync_activity(app_handle, true, "image", "正在同步图片…");
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
    emit_sync_activity(app_handle, false, "image", "图片已同步");
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
    handle: &ConnectionHandle,
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
        SignalMessage::TransferCancel { transfer_id, .. } => {
            image_session.cancel(&transfer_id);
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
                emit_sync_notice(app_handle, "收到的图片过大，已忽略。");
                return true;
            }

            emit_sync_activity(app_handle, true, "image", "正在同步图片…");
            if image_session
                .begin(
                    transfer_id,
                    hash_bytes,
                    width,
                    height,
                    total_bytes,
                    chunk_size,
                )
                .is_err()
            {
                emit_sync_notice(app_handle, "无法接收这张图片。");
            }
            true
        }
        SignalMessage::ClipboardImageEnd { transfer_id, hash } => {
            let hash_bytes = match decode_hash(&hash) {
                Some(value) => value,
                None => return true,
            };

            if let Some(completed) = image_session.end(&transfer_id, hash_bytes) {
                finalize_received_image(completed, dedup, clip_tx, peer_name, app_handle).await;
            } else {
                emit_sync_notice(app_handle, "图片接收未完成，已忽略。");
            }
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
                    return true;
                }
            };

            if png_bytes.len() > MAX_IMAGE_BYTES {
                emit_sync_notice(app_handle, "收到的图片过大，已忽略。");
                return true;
            }

            let payload_hash = *blake3::hash(&png_bytes).as_bytes();
            if payload_hash != hash_bytes {
                tracing::warn!("Image hash mismatch");
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
                    &receive_handle,
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
        connected: Arc<Mutex<bool>>,
        connection: Arc<Mutex<Option<ConnectionHandle>>>,
        connected_peer: Arc<Mutex<Option<crate::ConnectedPeerPayload>>>,
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

        let handle = new_handle(
            HandleTransport::Direct { tx: event_tx },
            connected.clone(),
            dedup.clone(),
        );

        *connected.lock().await = true;

        let receive_handle = handle.clone();
        tokio::spawn(async move {
            let mut peer_left = false;
            let mut image_session = ImageReceiveSession::new();

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
                            &receive_handle,
                        )
                        .await
                    }
                    ConnectionEvent::Binary(chunk) => {
                        let send_ack = |msg: SignalMessage| receive_handle.send_signal(msg);
                        image_session.ingest_chunk(&chunk, &send_ack);
                        true
                    }
                };

                if !should_continue {
                    peer_left = true;
                    break;
                }
            }

            tracing::warn!("Direct connection lost");

            if connection_generation.load(Ordering::SeqCst) != session_generation {
                return;
            }

            let was_connected = *connected.lock().await;
            *connected.lock().await = false;
            *connection.lock().await = None;
            *connected_peer.lock().await = None;

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
