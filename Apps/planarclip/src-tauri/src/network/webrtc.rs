use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Emitter, Manager};
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

    pub fn notify_peer_left(&self, local_peer_id: &str) {
        let msg = SignalMessage::PeerLeft {
            peer_id: local_peer_id.to_string(),
        };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = self.sig_tx.send(json);
        }
    }

    pub fn send_clipboard(&self, snapshot: &ClipboardSnapshot) {
        let is_connected = self
            .connected
            .try_lock()
            .map(|guard| *guard)
            .unwrap_or(false);
        if !is_connected {
            return;
        }

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
        connection: Arc<Mutex<Option<ConnectionHandle>>>,
        connected_peer: Arc<Mutex<Option<crate::ConnectedPeerPayload>>>,
        connection_generation: Arc<AtomicU64>,
        session_generation: u64,
        clip_tx: broadcast::Sender<ClipboardEvent>,
        app_handle: AppHandle,
    ) -> ConnectionHandle {
        let peer_name = conn.peer_name.clone();
        let peer_id = conn.peer_id.clone();
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
