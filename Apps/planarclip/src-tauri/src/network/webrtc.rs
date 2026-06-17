use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::clipboard::monitor::ClipboardMonitor;
use crate::clipboard::types::ClipboardSnapshot;
use crate::network::direct::DirectConnection;
use crate::network::protocol::SignalMessage;
use crate::network::signalling;
use crate::sync::dedup::DedupStore;

/// Shared handle for sending clipboard content to the remote peer.
/// Cheap to clone — can be shared between commands and the sync engine.
#[derive(Clone)]
pub struct ConnectionHandle {
    sig_tx: tokio::sync::mpsc::UnboundedSender<String>,
    connected: Arc<Mutex<bool>>,
    dedup: Arc<Mutex<DedupStore>>,
}

impl ConnectionHandle {
    /// Check whether we have an active connection.
    pub fn connected(&self) -> Arc<Mutex<bool>> {
        self.connected.clone()
    }

    /// Send a clipboard snapshot to the remote peer.
    /// Returns early if the content was already seen (dedup).
    pub fn send_clipboard(&self, snapshot: &ClipboardSnapshot) {
        if let ClipboardSnapshot::Text(ref text) = snapshot {
            let hash = snapshot.content_hash();
            let hash_hex = hex::encode(hash);

            // Don't echo back content we just received from remote,
            // and don't re-send content we already sent.
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

/// Manages the signalling connection lifecycle and incoming message processing.
pub struct ConnectionManager;

impl ConnectionManager {
    /// Connect to the signalling relay for the given room, spawn the receive
    /// loop, and return a clonable ConnectionHandle for sending.
    pub async fn connect(
        server_url: &str,
        room: &str,
        peer_id: &str,
        connected: Arc<Mutex<bool>>,
        clip_tx: broadcast::Sender<ClipboardSnapshot>,
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

        // Mark as connected
        *connected.lock().await = true;

        // Spawn receive loop
        tokio::spawn(async move {
            while let Some(msg) = sig_rx.recv().await {
                match msg {
                    SignalMessage::Clipboard { payload, hash } => {
                        // Parse the full hash from hex
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

                        // Dedup check
                        {
                            let mut d = dedup.lock().await;
                            if d.has_seen(&hash_bytes) {
                                continue;
                            }
                            d.mark_seen(hash_bytes);
                        }

                        tracing::info!("Received remote clipboard: {} chars", payload.len());

                        // Write to local clipboard (self-writing flag prevents re-broadcast)
                        ClipboardMonitor::write_clipboard(&payload);

                        // Broadcast to update UI
                        let snapshot = ClipboardSnapshot::Text(payload);
                        let _ = clip_tx.send(snapshot);
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

            // sig_rx closed = WS disconnected
            tracing::warn!("Signalling connection lost");
            *connected.lock().await = false;
        });

        Ok(handle)
    }

    /// Establish a connection over an existing TCP direct transport (post-handshake).
    ///
    /// Works identically to `connect()` but uses a `DirectConnection` instead of
    /// a WebSocket signalling relay.
    pub async fn connect_direct(
        conn: DirectConnection,
        connected: Arc<Mutex<bool>>,
        clip_tx: broadcast::Sender<ClipboardSnapshot>,
    ) -> ConnectionHandle {
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
                        let _ = clip_tx.send(snapshot);
                    }
                    SignalMessage::PeerJoined { .. } => {
                        // Not meaningful on direct connections; ignore.
                    }
                    SignalMessage::PeerLeft { .. } => {
                        tracing::info!("Direct peer disconnected");
                        *connected.lock().await = false;
                    }
                }
            }

            tracing::warn!("Direct connection lost");
            *connected.lock().await = false;
        });

        handle
    }
}
