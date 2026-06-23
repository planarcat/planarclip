use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::sync::{broadcast, Mutex};

use crate::clipboard::monitor::ClipboardMonitor;
use crate::clipboard::types::{ClipboardEvent, ClipboardSnapshot};
use crate::network::direct::DirectConnection;
use crate::network::protocol::SignalMessage;
use crate::network::signalling;
use crate::sync::dedup::DedupStore;

#[derive(Clone)]
pub struct ConnectionHandle {
    sig_tx: tokio::sync::mpsc::UnboundedSender<String>,
    connected: Arc<Mutex<bool>>,
    dedup: Arc<Mutex<DedupStore>>,
}

impl ConnectionHandle {
    pub fn connected(&self) -> Arc<Mutex<bool>> {
        self.connected.clone()
    }

    pub fn send_clipboard(&self, snapshot: &ClipboardSnapshot) {
        if let ClipboardSnapshot::Text(ref text) = snapshot {
            let hash = snapshot.content_hash();
            let hash_hex = hex::encode(hash);

            if let Ok(mut dedup) = self.dedup.try_lock() {
                if dedup.has_seen(&hash) {
                    return;
                }
                dedup.mark_seen(hash);
            }

            let msg = SignalMessage::Clipboard {
                payload: text.clone(),
                hash: hash_hex,
            };

            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = self.sig_tx.send(json);
            }
        }
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

        let handle = ConnectionHandle {
            sig_tx,
            connected: connected.clone(),
            dedup: dedup.clone(),
        };

        *connected.lock().await = true;

        tokio::spawn(async move {
            while let Some(msg) = sig_rx.recv().await {
                match msg {
                    SignalMessage::Clipboard { payload, hash } => {
                        let hash_bytes = match hex::decode(&hash) {
                            Ok(v) if v.len() == 32 => {
                                let mut arr = [0u8; 32];
                                arr.copy_from_slice(&v);
                                arr
                            }
                            _ => {
                                tracing::warn!("Invalid hash in clipboard message");
                                continue;
                            }
                        };

                        {
                            let mut d = dedup.lock().await;
                            if d.has_seen(&hash_bytes) {
                                continue;
                            }
                            d.mark_seen(hash_bytes);
                        }

                        tracing::info!("Received remote clipboard: {} chars", payload.len());
                        ClipboardMonitor::write_clipboard(&payload);
                        let snapshot = ClipboardSnapshot::Text(payload);
                        let _ = clip_tx.send(ClipboardEvent::remote(snapshot, "已配对设备".to_string()));
                    }
                    SignalMessage::PeerJoined { peer_id } => {
                        tracing::info!("Peer joined room: {}", peer_id);
                    }
                    SignalMessage::PeerLeft { peer_id } => {
                        tracing::info!("Peer left room: {}", peer_id);
                        *connected.lock().await = false;
                    }
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
        clip_tx: broadcast::Sender<ClipboardEvent>,
        app_handle: AppHandle,
    ) -> ConnectionHandle {
        let peer_name = conn.peer_name.clone();
        let sig_tx = conn.tx;
        let mut sig_rx = conn.rx;
        let dedup = Arc::new(Mutex::new(DedupStore::new(128)));

        let handle = ConnectionHandle {
            sig_tx,
            connected: connected.clone(),
            dedup: dedup.clone(),
        };

        *connected.lock().await = true;

        tokio::spawn(async move {
            let mut peer_left = false;

            while let Some(msg) = sig_rx.recv().await {
                match msg {
                    SignalMessage::Clipboard { payload, hash } => {
                        let hash_bytes = match hex::decode(&hash) {
                            Ok(v) if v.len() == 32 => {
                                let mut arr = [0u8; 32];
                                arr.copy_from_slice(&v);
                                arr
                            }
                            _ => {
                                tracing::warn!("Invalid hash in clipboard message");
                                continue;
                            }
                        };

                        {
                            let mut d = dedup.lock().await;
                            if d.has_seen(&hash_bytes) {
                                continue;
                            }
                            d.mark_seen(hash_bytes);
                        }

                        tracing::info!("Received remote clipboard: {} chars", payload.len());
                        ClipboardMonitor::write_clipboard(&payload);
                        let snapshot = ClipboardSnapshot::Text(payload);
                        let _ = clip_tx.send(ClipboardEvent::remote(snapshot, peer_name.clone()));
                    }
                    SignalMessage::PeerJoined { .. } => {}
                    SignalMessage::PeerLeft { .. } => {
                        tracing::info!("Direct peer disconnected");
                        peer_left = true;
                        break;
                    }
                }
            }

            tracing::warn!("Direct connection lost");
            *connected.lock().await = false;

            let message = if peer_name.is_empty() {
                "对方设备已断开连接，请重新连接。".to_string()
            } else {
                format!("{} 已断开连接，请重新连接。", peer_name)
            };
            let kind = if peer_left { "peer_disconnected" } else { "connection_lost" };
            let _ = app_handle.emit(
                "connection-ended",
                serde_json::json!({
                    "kind": kind,
                    "message": message,
                    "peer_name": peer_name,
                }),
            );
        });

        handle
    }
}
