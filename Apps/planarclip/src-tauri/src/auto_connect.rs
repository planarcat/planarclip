use crate::crypto::keys::KeyPair;
use crate::network::direct::{self, HandshakeError, InitiatorResult};
use crate::network::discovery::LanDevice;
use crate::network::webrtc::ConnectionManager;
use crate::storage::json::{AppConfig, TrustedPeerData};
use crate::{emit_connection_failed, store_connected_peer, upsert_trusted_peer, ConnectedPeerPayload};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, oneshot, Mutex};

use crate::clipboard::types::ClipboardEvent;
use crate::network::webrtc::ConnectionHandle;

/// Delay before surfacing auto-connect wait UI. Fast auto-accept paths finish silently.
const AUTO_CONNECT_UI_DELAY_MS: u64 = 400;

pub struct AutoConnectDeps {
    pub config: Arc<Mutex<AppConfig>>,
    pub key_pair: Arc<Mutex<Option<KeyPair>>>,
    pub connected: Arc<Mutex<bool>>,
    pub connected_peer: Arc<Mutex<Option<ConnectedPeerPayload>>>,
    pub connection: Arc<Mutex<Option<ConnectionHandle>>>,
    pub connection_generation: Arc<AtomicU64>,
    pub clip_tx: broadcast::Sender<ClipboardEvent>,
    pub pending_initiator: Arc<Mutex<Option<TcpStream>>>,
    pub pending_outbound: Arc<Mutex<Option<TcpStream>>>,
    pub outbound_abort: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub outbound_handshake_active: Arc<AtomicBool>,
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
    if deps.outbound_handshake_active.load(Ordering::SeqCst) {
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

fn lookup_trusted_peer_name(config: &AppConfig, peer_id: &str) -> String {
    config
        .trusted_peers
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|peer| peer.peer_id == peer_id)
        .map(|peer| peer.name.clone())
        .unwrap_or_else(|| "熟悉设备".to_string())
}

async fn establish_outbound_connection(
    deps: &AutoConnectDeps,
    app: &AppHandle,
    conn: direct::DirectConnection,
    ip: &str,
) -> bool {
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

    let session_generation = deps
        .connection_generation
        .fetch_add(1, Ordering::SeqCst)
        + 1;
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

async fn run_auto_outbound_handshake(
    deps: AutoConnectDeps,
    app: AppHandle,
    stream: TcpStream,
    ip: String,
    port: u16,
    peer_id: String,
    peer_name: String,
    emit_failures: bool,
) {
    deps.outbound_handshake_active
        .store(true, Ordering::SeqCst);
    struct ActiveGuard(Arc<AtomicBool>);
    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _active_guard = ActiveGuard(deps.outbound_handshake_active.clone());

    let outbound = Arc::new(Mutex::new(Some(stream)));
    let outbound_read = outbound.clone();
    let mut read_handle = tokio::spawn(async move {
        let mut guard = outbound_read.lock().await;
        let stream = guard
            .take()
            .ok_or(HandshakeError::Cancelled)?;
        direct::initiator_read_connect_response(stream).await
    });

    let (abort_tx, abort_rx) = oneshot::channel();
    *deps.outbound_abort.lock().await = Some(abort_tx);
    tokio::pin!(abort_rx);

    let handshake_result: Result<(InitiatorResult, bool), HandshakeError> = tokio::select! {
        _ = &mut abort_rx => {
            if let Some(stream) = outbound.lock().await.take() {
                direct::initiator_abort(stream).await;
            }
            read_handle.abort();
            Err(HandshakeError::Cancelled)
        }
        join_result = &mut read_handle => {
            match join_result {
                Ok(Ok(handshake)) => Ok((handshake, false)),
                Ok(Err(error)) => Err(error),
                Err(_) => Err(HandshakeError::Cancelled),
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(AUTO_CONNECT_UI_DELAY_MS)) => {
            let _ = app.emit(
                "outbound-connection-pending",
                serde_json::json!({
                    "peer_id": peer_id,
                    "peer_name": peer_name,
                    "peer_ip": ip,
                    "peer_port": port,
                    "source": "auto_connect",
                }),
            );
            tokio::select! {
                _ = &mut abort_rx => {
                    if let Some(stream) = outbound.lock().await.take() {
                        direct::initiator_abort(stream).await;
                    }
                    read_handle.abort();
                    Err(HandshakeError::Cancelled)
                }
                join_result = &mut read_handle => match join_result {
                    Ok(Ok(handshake)) => Ok((handshake, true)),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(HandshakeError::Cancelled),
                },
            }
        }
    };

    *deps.outbound_abort.lock().await = None;

    let (handshake, ui_was_shown) = match handshake_result {
        Ok(value) => value,
        Err(error) => {
            if emit_failures {
                emit_connection_failed(&app, &error);
            } else {
                tracing::debug!(
                    "Auto-connect outbound handshake aborted or failed: {}",
                    error.user_message()
                );
            }
            return;
        }
    };

    match handshake {
        InitiatorResult::Connected(conn) => {
            let _ = establish_outbound_connection(&deps, &app, conn, &ip).await;
        }
        InitiatorResult::AwaitingCode { stream } => {
            crate::store_pending_initiator_stream(
                deps.pending_initiator.clone(),
                &app,
                app.state::<crate::AppState>().inner().clone(),
                stream,
            )
            .await;
            let _ = app.emit(
                "pairing-code-needed",
                serde_json::json!({
                    "peer_ip": ip,
                    "peer_id": peer_id,
                    "peer_name": peer_name,
                    "peer_port": port,
                    "source": "auto_connect",
                }),
            );
            if ui_was_shown {
                tracing::info!(
                    "Auto-connect to {} entered pairing-code flow after showing wait UI",
                    peer_id
                );
            }
        }
    }
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

    let (device_name, key_pair, peer_name) = {
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
        (
            config.device_name.clone(),
            kp,
            lookup_trusted_peer_name(&config, peer_id),
        )
    };

    tracing::info!(
        "Auto-connecting to trusted peer {} at {}:{}",
        peer_id,
        ip,
        port
    );

    let stream = match direct::initiator_send_connect_request(
        ip,
        port,
        &device_name,
        &key_pair,
        false,
    )
    .await
    {
        Ok(stream) => stream,
        Err(error) => {
            if emit_failures {
                emit_connection_failed(app, &error);
            } else {
                tracing::debug!(
                    "Auto-connect to trusted peer {} failed before handshake: {}",
                    peer_id,
                    error.user_message()
                );
            }
            return false;
        }
    };

    let deps = AutoConnectDeps {
        config: deps.config.clone(),
        key_pair: deps.key_pair.clone(),
        connected: deps.connected.clone(),
        connected_peer: deps.connected_peer.clone(),
        connection: deps.connection.clone(),
        connection_generation: deps.connection_generation.clone(),
        clip_tx: deps.clip_tx.clone(),
        pending_initiator: deps.pending_initiator.clone(),
        pending_outbound: deps.pending_outbound.clone(),
        outbound_abort: deps.outbound_abort.clone(),
        outbound_handshake_active: deps.outbound_handshake_active.clone(),
        pending_connection_request: deps.pending_connection_request.clone(),
        tcp_port: deps.tcp_port,
    };
    let app = app.clone();
    let ip = ip.to_string();
    let peer_id = peer_id.to_string();

    tauri::async_runtime::spawn(async move {
        run_auto_outbound_handshake(
            deps,
            app,
            stream,
            ip,
            port,
            peer_id,
            peer_name,
            emit_failures,
        )
        .await;
    });

    true
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
