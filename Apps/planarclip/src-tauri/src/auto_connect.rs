use crate::crypto::keys::KeyPair;
use crate::network::direct::{self, InitiatorResult};
use crate::network::discovery::LanDevice;
use crate::network::webrtc::ConnectionManager;
use crate::storage::json::{AppConfig, TrustedPeerData};
use crate::{store_connected_peer, upsert_trusted_peer, ConnectedPeerPayload};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, Mutex};

use crate::clipboard::types::ClipboardEvent;
use crate::network::webrtc::ConnectionHandle;

pub struct AutoConnectDeps {
    pub config: Arc<Mutex<AppConfig>>,
    pub key_pair: Arc<Mutex<Option<KeyPair>>>,
    pub connected: Arc<Mutex<bool>>,
    pub connected_peer: Arc<Mutex<Option<ConnectedPeerPayload>>>,
    pub connection: Arc<Mutex<Option<ConnectionHandle>>>,
    pub connection_generation: Arc<AtomicU64>,
    pub clip_tx: broadcast::Sender<ClipboardEvent>,
    pub pending_initiator: Arc<Mutex<Option<TcpStream>>>,
    pub pending_connection_request: Arc<Mutex<Option<crate::window::ConnectionRequestPayload>>>,
    pub tcp_port: u16,
}

pub fn auto_connect_trusted_enabled(config: &AppConfig) -> bool {
    config.auto_connect_trusted.unwrap_or(false)
}

pub async fn can_initiate_outbound(deps: &AutoConnectDeps) -> bool {
    if *deps.connected.lock().await {
        return false;
    }
    if deps.pending_initiator.lock().await.is_some() {
        return false;
    }
    if deps.pending_connection_request.lock().await.is_some() {
        return false;
    }
    true
}

fn trusted_peer_ids(config: &AppConfig) -> HashSet<String> {
    config
        .trusted_peers
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|peer| peer.peer_id.clone())
        .collect()
}

pub async fn attempt_connect_trusted_peer(
    deps: &AutoConnectDeps,
    app: &AppHandle,
    ip: &str,
    port: u16,
    peer_id: &str,
    emit_failures: bool,
) -> bool {
    if !auto_connect_trusted_enabled(&*deps.config.lock().await) {
        return false;
    }
    if !can_initiate_outbound(deps).await {
        return false;
    }

    let (device_name, key_pair) = {
        let config = deps.config.lock().await;
        if !config
            .trusted_peers
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|peer| peer.peer_id == peer_id)
        {
            return false;
        }
        let kp = deps.key_pair.lock().await;
        let Some(kp) = kp.clone() else {
            tracing::warn!("Auto-connect skipped: key pair not initialized");
            return false;
        };
        (config.device_name.clone(), kp)
    };

    tracing::info!(
        "Auto-connecting to trusted peer {} at {}:{}",
        peer_id,
        ip,
        port
    );

    match direct::initiator_connect(ip, port, &device_name, &key_pair, false).await {
        Ok(InitiatorResult::Connected(conn)) => {
            let peer_name = conn.peer_name.clone();
            let peer_id = conn.peer_id.clone();
            let peer_pk = conn.peer_public_key.clone();

            {
                let mut config = deps.config.lock().await;
                upsert_trusted_peer(
                    &mut config,
                    TrustedPeerData {
                        name: peer_name.clone(),
                        public_key: peer_pk,
                        peer_id: peer_id.clone(),
                        last_ip: Some(ip.to_string()),
                        auto_accept: None,
                    },
                );
                crate::storage::json::save_config(&config);
            }

            let session_generation = deps.connection_generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let handle = ConnectionManager::connect_direct(
                conn,
                deps.connected.clone(),
                deps.connection.clone(),
                deps.connected_peer.clone(),
                deps.connection_generation.clone(),
                session_generation,
                deps.clip_tx.clone(),
                app.clone(),
            )
            .await;
            *deps.connection.lock().await = Some(handle);
            store_connected_peer(&deps.connected_peer, peer_name.clone(), peer_id.clone()).await;

            let _ = app.emit(
                "connection-established",
                serde_json::json!({
                    "peer_name": peer_name,
                    "peer_id": peer_id,
                    "is_reconnect": true,
                }),
            );
            true
        }
        Ok(InitiatorResult::AwaitingCode { mut stream }) => {
            let _ = stream.shutdown().await;
            tracing::warn!(
                "Auto-connect to trusted peer {} unexpectedly required pairing code",
                peer_id
            );
            false
        }
        Err(error) => {
            if emit_failures {
                crate::emit_connection_failed(app, &error);
            } else {
                tracing::debug!(
                    "Auto-connect to trusted peer {} failed: {}",
                    peer_id,
                    error.user_message()
                );
            }
            false
        }
    }
}

fn collect_startup_targets(
    trusted_peers: &[TrustedPeerData],
    lan_devices: &[LanDevice],
    fallback_port: u16,
) -> Vec<(String, u16, String)> {
    let mut lan_by_peer = HashMap::<String, (&str, u16)>::new();
    for device in lan_devices {
        lan_by_peer
            .entry(device.peer_id.clone())
            .or_insert((device.ip.as_str(), device.port));
    }

    let mut targets = Vec::new();
    for peer in trusted_peers {
        if let Some((ip, port)) = lan_by_peer.get(&peer.peer_id).copied() {
            targets.push((ip.to_string(), port, peer.peer_id.clone()));
            continue;
        }
        if let Some(last_ip) = peer
            .last_ip
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            targets.push((last_ip.to_string(), fallback_port, peer.peer_id.clone()));
        }
    }
    targets
}

pub async fn auto_connect_trusted_on_startup(
    deps: AutoConnectDeps,
    app: AppHandle,
    lan_devices: Arc<Mutex<Vec<LanDevice>>>,
) {
    if !auto_connect_trusted_enabled(&*deps.config.lock().await) {
        return;
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let trusted_peers = {
        let config = deps.config.lock().await;
        if !auto_connect_trusted_enabled(&config) {
            return;
        }
        config.trusted_peers.clone().unwrap_or_default()
    };

    if trusted_peers.is_empty() {
        return;
    }

    let lan_snapshot = lan_devices.lock().await.clone();
    let targets = collect_startup_targets(&trusted_peers, &lan_snapshot, deps.tcp_port);
    if targets.is_empty() {
        tracing::info!("Auto-connect on startup: no reachable trusted peers yet");
        return;
    }

    for (ip, port, peer_id) in targets {
        if !can_initiate_outbound(&deps).await {
            break;
        }
        if attempt_connect_trusted_peer(&deps, &app, &ip, port, &peer_id, false).await {
            break;
        }
    }
}

pub async fn maybe_auto_connect_discovered_device(
    deps: &AutoConnectDeps,
    app: &AppHandle,
    device: &LanDevice,
) {
    let enabled = auto_connect_trusted_enabled(&*deps.config.lock().await);
    if !enabled {
        return;
    }

    let trusted_ids = trusted_peer_ids(&*deps.config.lock().await);
    if !trusted_ids.contains(&device.peer_id) {
        return;
    }

    let _ = attempt_connect_trusted_peer(
        deps,
        app,
        &device.ip,
        device.port,
        &device.peer_id,
        false,
    )
    .await;
}
