use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};

mod app_profile;
mod auto_connect;
mod clipboard;
mod crypto;
mod logging;
mod network;
mod platform;
mod storage;
mod sync;
mod tray;
mod window;

use clipboard::monitor::{ClipboardDedupBaseline, ClipboardMonitor};
use crate::app_profile::APP_DISPLAY_NAME;
use tauri_plugin_autostart::ManagerExt;
use clipboard::types::{ClipboardEvent, ClipboardHistoryEntry, ClipboardOrigin};
use crypto::keys::{peer_id_from_public_key, KeyPair};
use network::direct::{self, InitiatorResult, ListenerEvent};
use network::discovery::{self, DiscoveryEvent, LanDevice};
use network::sessions::ConnectionRegistry;
use network::webrtc::{ConnectionHandle, ConnectionManager};
use storage::json::{self as storage_json, AppConfig, KeyPairData, PeerData, TrustedPeerData};

const SIGNALLING_SERVER: &str = "ws://localhost:8765";
const DEFAULT_DEVICE_NAME: &str = "我的设备";
const DEFAULT_UI_COLOR_SCHEME: &str = "dark";
const DEFAULT_UI_THEME_COLOR: &str = "cyan";
const CLIPBOARD_HISTORY_LIMIT_OPTIONS: [usize; 5] = [25, 50, 100, 200, 500];
const DEFAULT_CLIPBOARD_HISTORY_LIMIT: usize = 100;
pub(crate) const MAX_CONNECTIONS: usize = 5;
const DEFAULT_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

fn default_device_name() -> String {
    let host_name = hostname::get()
        .ok()
        .and_then(|value| value.into_string().ok())
        .map(|value| {
            value
                .trim()
                .trim_end_matches('.')
                .trim_end_matches(".local")
                .chars()
                .take(24)
                .collect::<String>()
        })
        .filter(|value| !value.is_empty());

    host_name.unwrap_or_else(|| DEFAULT_DEVICE_NAME.to_string())
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub key_pair: Arc<Mutex<Option<KeyPair>>>,
    pub connections: Arc<Mutex<ConnectionRegistry>>,
    pub transfer_slots: Arc<sync::transfer_limit::TransferSlotLimiter>,
    pub connection_generation: Arc<AtomicU64>,
    pub clipboard_monitor_generation: Arc<AtomicU64>,
    pub clipboard_dedup_baseline: Arc<Mutex<Option<ClipboardDedupBaseline>>>,
    pub clip_tx: broadcast::Sender<ClipboardEvent>,
    pub clipboard_history: Arc<Mutex<Vec<ClipboardHistoryEntry>>>,
    pub lan_devices: Arc<Mutex<Vec<LanDevice>>>,
    pub peer_offline_cooldown: Arc<Mutex<HashMap<String, i64>>>,
    pub pending_initiator: Arc<Mutex<Option<TcpStream>>>,
    pub pending_outbound: Arc<Mutex<Option<TcpStream>>>,
    pub outbound_abort: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub outbound_handshake_active: Arc<AtomicBool>,
    pub pending_accept_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub pending_reject_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    pub pending_incoming_timeout_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    pub pending_connection_request: Arc<Mutex<Option<window::ConnectionRequestPayload>>>,
    pub pairing_session_code: Arc<Mutex<Option<Arc<Mutex<String>>>>>,
    pub pairing_code_expires_at: Arc<Mutex<Option<i64>>>,
    pub pending_responder_submit_tx:
        Arc<Mutex<Option<mpsc::Sender<(String, oneshot::Sender<Result<(), direct::HandshakeError>>)>>>>,
    /// Set by the frontend after first paint; used to defer showing a newly built main window.
    pub main_window_ui_ready: Arc<std::sync::atomic::AtomicBool>,
    /// When Some, a new main WebView was built and is waiting for UI ready (or timeout) before show.
    pub main_window_reveal_steal_focus: Arc<std::sync::Mutex<Option<bool>>>,
    pub broadcast_state: Arc<Mutex<BroadcastState>>,
    pub broadcast_handles: Arc<Mutex<Option<BroadcastHandles>>>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "state")]
pub enum BroadcastState {
    Active { port: u16 },
    PortConflict { port: u16 },
    Inactive { port: u16, reason: String },
}

pub(crate) struct BroadcastHandles {
    discovery_daemon: Option<network::discovery::ServiceDaemon>,
    discovery_task: tauri::async_runtime::JoinHandle<()>,
    presence_task: tauri::async_runtime::JoinHandle<()>,
    listener_task: Option<tauri::async_runtime::JoinHandle<()>>,
    listener_event_task: Option<tauri::async_runtime::JoinHandle<()>>,
}

/// Start mDNS discovery + TCP listener + presence refresh + auto-connect for `port`.
/// Returns handles so the caller can stop the broadcast later (e.g. on port change).
async fn start_broadcast(app: &tauri::AppHandle, port: u16, register_self: bool, with_listener: bool) -> Result<BroadcastHandles, String> {
    let state = app.state::<AppState>();
    let app_key_pair = state.key_pair.clone();
    let key_pair = state
        .key_pair
        .lock()
        .await
        .clone()
        .ok_or_else(|| "key pair not ready".to_string())?;
    let device_name = normalize_stored_device_name(&state.config.lock().await.device_name);
    let peer_id = key_pair.fingerprint();
    let local_ips = discovery::local_ips();
    let lan_devices = state.lan_devices.clone();
    let pending_accept_tx = state.pending_accept_tx.clone();
    let pending_reject_tx = state.pending_reject_tx.clone();
    let connections = state.connections.clone();
    let connection_generation = state.connection_generation.clone();
    let config = state.config.clone();
    let clip_tx = state.clip_tx.clone();
    let app_handle = app.clone();

    let (discovery_tx, mut discovery_rx) = mpsc::unbounded_channel::<DiscoveryEvent>();
    let daemon = discovery::start_discovery(&device_name, &peer_id, port, register_self, discovery_tx)
        .map_err(|e| format!("mDNS discovery failed: {e}"))?;

    let discovery_task = {
        let lan_devices = lan_devices.clone();
        let local_ips = local_ips.clone();
        let app_handle = app_handle.clone();
        let verify_key_pair = app_key_pair.clone();
        let verify_cooldown = state.peer_offline_cooldown.clone();
        let auto_connect_deps = auto_connect::AutoConnectDeps {
            config: config.clone(),
            key_pair: app_key_pair.clone(),
            connections: connections.clone(),
            connection_generation: connection_generation.clone(),
            clip_tx: clip_tx.clone(),
            pending_initiator: state.pending_initiator.clone(),
            pending_outbound: state.pending_outbound.clone(),
            outbound_abort: state.outbound_abort.clone(),
            outbound_handshake_active: state.outbound_handshake_active.clone(),
            pending_connection_request: state.pending_connection_request.clone(),
            tcp_port: port,
        };
        tauri::async_runtime::spawn(async move {
            while let Some(event) = discovery_rx.recv().await {
                match event {
                    DiscoveryEvent::Added(candidate) => {
                        let verify_key_pair = verify_key_pair.clone();
                        let verify_cooldown = verify_cooldown.clone();
                        let verify_lan_devices = lan_devices.clone();
                        let verify_app = app_handle.clone();
                        let auto_connect_deps = auto_connect_deps.clone();
                        let local_ips = local_ips.clone();
                        tauri::async_runtime::spawn(async move {
                            if local_ips.contains(&candidate.ip) {
                                return; // 本机另一实例,不显示
                            }
                            if is_peer_in_offline_cooldown(&verify_cooldown, &candidate.peer_id)
                                .await
                            {
                                // Peer was offline recently; skip presence probe but still
                                // attempt auto-connect (mDNS announce = likely back online).
                                auto_connect::maybe_auto_connect_discovered_device(
                                    &auto_connect_deps,
                                    &verify_app,
                                    &candidate,
                                )
                                .await;
                                return;
                            }
                            let Some(querier_peer_id) =
                                local_querier_peer_id(&verify_key_pair).await
                            else {
                                return;
                            };
                            let probe_ports =
                                app_profile::tcp_probe_port_candidates(port);
                            let Some(presence) = direct::probe_planarclip_presence(
                                &candidate.ip,
                                &probe_ports,
                                &querier_peer_id,
                                Some(&candidate.peer_id),
                                LAN_PROBE_TIMEOUT,
                            )
                            .await
                            else {
                                return;
                            };

                            let mut confirmed = candidate;
                            apply_presence_probe(&mut confirmed, &presence);
                            tracing::info!(
                                "LAN device confirmed: {} ({}) via presence probe",
                                confirmed.name,
                                confirmed.ip
                            );
                            if !upsert_presence_confirmed_device(
                                &verify_lan_devices,
                                confirmed.clone(),
                                &verify_app,
                            )
                            .await
                            {
                                return;
                            }

                            auto_connect::maybe_auto_connect_discovered_device(
                                &auto_connect_deps,
                                &verify_app,
                                &confirmed,
                            )
                            .await;
                        });
                    }
                    DiscoveryEvent::Removed { service_fullname } => {
                        let mut devices = lan_devices.lock().await;
                        let before = devices.len();
                        devices.retain(|d| !lan_device_matches_removal(d, &service_fullname));
                        if devices.len() != before {
                            tracing::info!("LAN device removed: {}", service_fullname);
                            let updated = devices.clone();
                            drop(devices);
                            let _ = app_handle.emit("lan-devices-changed", &updated);
                        }
                    }
                }
            }
        })
    };

    let presence_task = {
        let probe_lan_devices = lan_devices.clone();
        let probe_connections = connections.clone();
        let probe_config = config.clone();
        let probe_key_pair = app_key_pair.clone();
        let probe_cooldown = state.peer_offline_cooldown.clone();
        let probe_app_handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            loop {
                refresh_lan_presence(
                    &probe_config,
                    &probe_key_pair,
                    &probe_lan_devices,
                    &probe_connections,
                    &probe_cooldown,
                    port,
                    &probe_app_handle,
                )
                .await;
                tokio::time::sleep(LAN_PROBE_INTERVAL).await;
            }
        })
    };

    {
        let startup_deps = auto_connect::AutoConnectDeps {
            config: config.clone(),
            key_pair: state.key_pair.clone(),
            connections: connections.clone(),
            connection_generation: connection_generation.clone(),
            clip_tx: clip_tx.clone(),
            pending_initiator: state.pending_initiator.clone(),
            pending_outbound: state.pending_outbound.clone(),
            outbound_abort: state.outbound_abort.clone(),
            outbound_handshake_active: state.outbound_handshake_active.clone(),
            pending_connection_request: state.pending_connection_request.clone(),
            tcp_port: port,
        };
        let startup_app_handle = app_handle.clone();
        let startup_lan_devices = lan_devices.clone();
        tauri::async_runtime::spawn(async move {
            auto_connect::auto_connect_trusted_on_startup(
                startup_deps,
                startup_app_handle,
                startup_lan_devices,
            )
            .await;
        });
    }

    let (listener_task, listener_event_task) = if with_listener {
        let (listener_tx, mut listener_rx) = mpsc::unbounded_channel::<ListenerEvent>();
        let presence_responder = direct::PresenceResponder {
            config: config.clone(),
            key_pair: app_key_pair.clone(),
        };
        let listener_task = tauri::async_runtime::spawn(async move {
            if let Err(e) = direct::run_listener(port, presence_responder, listener_tx).await {
                tracing::error!("TCP listener error: {}", e);
            }
        });

        let listener_event_task = {
            let app_handle = app_handle.clone();
            let connections_listener = connections.clone();
            let connection_generation_listener = connection_generation.clone();
            let clip_tx = clip_tx.clone();
            let pending_accept_tx = pending_accept_tx.clone();
            let pending_reject_tx = pending_reject_tx.clone();
            let key_pair = key_pair.clone();
            let config = config.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = listener_rx.recv().await {
                    match event {
                        ListenerEvent::Incoming(req) => {
                            handle_incoming_connection(
                                req,
                                key_pair.clone(),
                                config.clone(),
                                app_handle.clone(),
                                connections_listener.clone(),
                                connection_generation_listener.clone(),
                                clip_tx.clone(),
                                pending_accept_tx.clone(),
                                pending_reject_tx.clone(),
                            )
                            .await;
                        }
                    }
                }
            })
        };

        (Some(listener_task), Some(listener_event_task))
    } else {
        (None, None)
    };

    Ok(BroadcastHandles {
        discovery_daemon: Some(daemon),
        discovery_task,
        presence_task,
        listener_task,
        listener_event_task,
    })
}

fn stop_broadcast(handles: BroadcastHandles) {
    if let Some(daemon) = handles.discovery_daemon {
        let _ = daemon.shutdown();
    }
    handles.discovery_task.abort();
    handles.presence_task.abort();
    if let Some(task) = handles.listener_task {
        task.abort();
    }
    if let Some(task) = handles.listener_event_task {
        task.abort();
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct UiSettingsPayload {
    color_scheme: String,
    theme_color: String,
    device_name: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct StartupSettingsPayload {
    launch_at_startup: bool,
    silent_start: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
struct AppBehaviorSettingsPayload {
    system_notifications_enabled: bool,
    close_window_action: String,
}

const DEFAULT_CLOSE_WINDOW_ACTION: &str = "tray";

fn normalized_close_window_action(value: Option<&str>) -> &'static str {
    match value {
        Some("exit") => "exit",
        Some("tray") => "tray",
        _ => DEFAULT_CLOSE_WINDOW_ACTION,
    }
}

fn app_behavior_from_config(config: &storage_json::AppConfig) -> AppBehaviorSettingsPayload {
    AppBehaviorSettingsPayload {
        system_notifications_enabled: config.system_notifications_enabled.unwrap_or(true),
        close_window_action: normalized_close_window_action(config.close_window_action.as_deref())
            .to_string(),
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct ConnectionSettingsPayload {
    auto_connect_trusted: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
struct SyncSettingsPayload {
    sync_images: bool,
    sync_files: bool,
    max_file_mb: u32,
    auto_sync_clipboard: bool,
    sync_files_save_enabled: bool,
    sync_files_save_dir: String,
    sync_files_save_dir_is_default: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
struct ClipboardSettingsPayload {
    history_limit: usize,
    view_mode: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct ShellBootstrapPayload {
    ui_settings: UiSettingsPayload,
    status: String,
    pairing_code: String,
    connected_peers: Vec<ConnectedPeerPayload>,
    pending_connection_request: Option<window::ConnectionRequestPayload>,
    broadcast_state: BroadcastState,
}

#[derive(Clone, Debug, serde::Serialize)]
struct ShellDeferredPayload {
    clipboard_history: Vec<ClipboardHistoryEntry>,
    lan_devices: Vec<network::discovery::LanDevice>,
    trusted_peers: Vec<TrustedPeerPayload>,
    clipboard_settings: ClipboardSettingsPayload,
    auto_sync_clipboard: bool,
}

const DEFAULT_CLIPBOARD_VIEW_MODE: &str = "grid";

#[derive(Clone, Debug, serde::Serialize)]
pub struct ConnectedPeerPayload {
    peer_name: String,
    peer_id: String,
}

impl ConnectedPeerPayload {
    pub(crate) fn peer_id(&self) -> &str {
        &self.peer_id
    }

    pub(crate) fn peer_name(&self) -> &str {
        &self.peer_name
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct TrustedPeerPayload {
    name: String,
    peer_id: String,
    last_ip: Option<String>,
    auto_accept: bool,
}

fn normalized_color_scheme(value: &str) -> Option<&'static str> {
    match value {
        "light" => Some("light"),
        "dark" => Some("dark"),
        "system" => Some("system"),
        _ => None,
    }
}

fn normalized_theme_color(value: &str) -> Option<&'static str> {
    match value {
        "cyan" => Some("cyan"),
        "violet" => Some("violet"),
        "emerald" => Some("emerald"),
        "rose" => Some("rose"),
        _ => None,
    }
}

fn normalize_stored_device_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("My Device") || trimmed == DEFAULT_DEVICE_NAME {
        default_device_name()
    } else {
        trimmed.to_string()
    }
}

fn lan_device_matches_removal(device: &LanDevice, service_fullname: &str) -> bool {
    if !device.service_fullname.is_empty() {
        return device.service_fullname == service_fullname;
    }

    // Entries discovered before service_fullname was tracked still match by peer id.
    service_fullname.starts_with(&app_profile::mdns_service_fullname_prefix(&device.peer_id))
}

const LAN_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const LAN_PROBE_INTERVAL: Duration = Duration::from_secs(8);
const LAN_PRESENCE_REFRESH_DELAY: Duration = Duration::from_millis(350);
const PEER_OFFLINE_COOLDOWN: Duration = Duration::from_secs(30);

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

async fn local_querier_peer_id(key_pair: &Arc<Mutex<Option<KeyPair>>>) -> Option<String> {
    key_pair
        .lock()
        .await
        .as_ref()
        .map(|key_pair| key_pair.fingerprint())
}

async fn is_peer_in_offline_cooldown(
    cooldown: &Arc<Mutex<HashMap<String, i64>>>,
    peer_id: &str,
) -> bool {
    let Some(until) = cooldown.lock().await.get(peer_id).copied() else {
        return false;
    };
    now_unix_ms() < until
}

pub(crate) async fn mark_peer_offline_cooldown(state: &AppState, peer_id: &str) {
    let peer_id = peer_id.trim();
    if peer_id.is_empty() {
        return;
    }
    let until = now_unix_ms() + PEER_OFFLINE_COOLDOWN.as_millis() as i64;
    state
        .peer_offline_cooldown
        .lock()
        .await
        .insert(peer_id.to_string(), until);
}

fn apply_presence_probe(device: &mut LanDevice, presence: &direct::PresenceProbeResult) {
    device.port = presence.port;
    device.last_presence_at = Some(now_unix_ms());
    if !presence.device_name.trim().is_empty() {
        device.name = presence.device_name.clone();
    }
}

async fn upsert_presence_confirmed_device(
    lan_devices: &Arc<Mutex<Vec<LanDevice>>>,
    device: LanDevice,
    app: &tauri::AppHandle,
) -> bool {
    let mut devices = lan_devices.lock().await;
    if let Some(existing) = devices.iter_mut().find(|entry| entry.peer_id == device.peer_id) {
        *existing = device;
    } else {
        devices.push(device);
    }

    let updated = devices.clone();
    drop(devices);
    let _ = app.emit("lan-devices-changed", &updated);
    true
}

async fn discover_trusted_peers_by_presence_probe(
    config: &Arc<Mutex<AppConfig>>,
    key_pair: &Arc<Mutex<Option<KeyPair>>>,
    lan_devices: &Arc<Mutex<Vec<LanDevice>>>,
    peer_offline_cooldown: &Arc<Mutex<HashMap<String, i64>>>,
    tcp_port: u16,
    app: &tauri::AppHandle,
) {
    let Some(querier_peer_id) = local_querier_peer_id(key_pair).await else {
        return;
    };

    let (trusted_peers, known_peer_ids) = {
        let config_guard = config.lock().await;
        let peers = config_guard.trusted_peers.clone().unwrap_or_default();
        let known_peer_ids = lan_devices
            .lock()
            .await
            .iter()
            .map(|device| device.peer_id.clone())
            .collect::<HashSet<_>>();
        (peers, known_peer_ids)
    };

    let probe_ports = app_profile::tcp_probe_port_candidates(tcp_port);
    let mut discovered = Vec::new();

    for peer in trusted_peers {
        if known_peer_ids.contains(&peer.peer_id) {
            continue;
        }
        if is_peer_in_offline_cooldown(peer_offline_cooldown, &peer.peer_id).await {
            continue;
        }
        let Some(last_ip) = peer
            .last_ip
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };

        let Some(presence) = direct::probe_planarclip_presence(
            last_ip,
            &probe_ports,
            &querier_peer_id,
            Some(&peer.peer_id),
            LAN_PROBE_TIMEOUT,
        )
        .await
        else {
            continue;
        };

        let mut device = LanDevice {
            name: peer.name.clone(),
            peer_id: peer.peer_id.clone(),
            ip: last_ip.to_string(),
            host_name: String::new(),
            port: presence.port,
            service_fullname: String::new(),
            last_presence_at: Some(now_unix_ms()),
        };
        apply_presence_probe(&mut device, &presence);
        discovered.push(device);
    }

    for device in discovered {
        tracing::info!(
            "Familiar peer {} confirmed at {}:{} via presence probe (mDNS miss)",
            device.name,
            device.ip,
            device.port
        );
        upsert_presence_confirmed_device(lan_devices, device, app).await;
    }
}

pub(crate) async fn refresh_lan_presence(
    config: &Arc<Mutex<AppConfig>>,
    key_pair: &Arc<Mutex<Option<KeyPair>>>,
    lan_devices: &Arc<Mutex<Vec<LanDevice>>>,
    connections: &Arc<Mutex<ConnectionRegistry>>,
    peer_offline_cooldown: &Arc<Mutex<HashMap<String, i64>>>,
    tcp_port: u16,
    app: &tauri::AppHandle,
) {
    discover_trusted_peers_by_presence_probe(
        config,
        key_pair,
        lan_devices,
        peer_offline_cooldown,
        tcp_port,
        app,
    )
    .await;
    reconcile_lan_devices(
        key_pair,
        lan_devices,
        connections,
        peer_offline_cooldown,
        tcp_port,
        app,
    )
    .await;
}

pub(crate) fn spawn_lan_presence_refresh(state: &AppState, app: &tauri::AppHandle) {
    let config = state.config.clone();
    let key_pair = state.key_pair.clone();
    let lan_devices = state.lan_devices.clone();
    let connections = state.connections.clone();
    let peer_offline_cooldown = state.peer_offline_cooldown.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(LAN_PRESENCE_REFRESH_DELAY).await;
        let tcp_port = config
            .lock()
            .await
            .tcp_port
            .unwrap_or(app_profile::DEFAULT_TCP_PORT);
        refresh_lan_presence(
            &config,
            &key_pair,
            &lan_devices,
            &connections,
            &peer_offline_cooldown,
            tcp_port,
            &app,
        )
        .await;
    });
}

fn peer_disconnected_message(peer_name: &str) -> String {
    if peer_name.trim().is_empty() {
        "设备 已断开连接".into()
    } else {
        format!("{} 已断开连接", peer_name.trim())
    }
}

fn peer_offline_message(peer_name: &str) -> String {
    if peer_name.trim().is_empty() {
        "对方设备 已下线".into()
    } else {
        format!("{} 已下线", peer_name.trim())
    }
}

async fn peer_still_reachable(state: &AppState, peer_id: &str, tcp_port: u16) -> bool {
    let Some(querier_peer_id) = local_querier_peer_id(&state.key_pair).await else {
        return false;
    };

    let probe_ports = app_profile::tcp_probe_port_candidates(tcp_port);

    let lan_target = {
        let devices = state.lan_devices.lock().await;
        devices
            .iter()
            .find(|device| device.peer_id == peer_id)
            .map(|device| (device.ip.clone(), device.port))
    };

    if let Some((ip, port)) = lan_target {
        return direct::probe_planarclip_presence_resilient(
            &ip,
            port,
            &probe_ports,
            &querier_peer_id,
            Some(peer_id),
            LAN_PROBE_TIMEOUT,
        )
        .await
        .is_some();
    }

    let trusted_ip = {
        let config = state.config.lock().await;
        config
            .trusted_peers
            .as_ref()
            .and_then(|peers| peers.iter().find(|peer| peer.peer_id == peer_id))
            .and_then(|peer| peer.last_ip.clone())
            .filter(|ip| !ip.trim().is_empty())
    };

    if let Some(ip) = trusted_ip {
        return direct::probe_planarclip_presence(
            &ip,
            &probe_ports,
            &querier_peer_id,
            Some(peer_id),
            LAN_PROBE_TIMEOUT,
        )
        .await
        .is_some();
    }

    false
}

pub(crate) async fn resolve_connection_ended_message(
    state: &AppState,
    peer_id: &str,
    peer_name: &str,
    peer_left: bool,
    tcp_port: u16,
) -> (String, &'static str) {
    if peer_left {
        return (peer_disconnected_message(peer_name), "peer_disconnected");
    }

    if peer_still_reachable(state, peer_id, tcp_port).await {
        return (peer_disconnected_message(peer_name), "connection_lost");
    }

    (peer_offline_message(peer_name), "peer_offline")
}

async fn reconcile_lan_devices(
    key_pair: &Arc<Mutex<Option<KeyPair>>>,
    lan_devices: &Arc<Mutex<Vec<LanDevice>>>,
    connections: &Arc<Mutex<ConnectionRegistry>>,
    _peer_offline_cooldown: &Arc<Mutex<HashMap<String, i64>>>,
    tcp_port: u16,
    app: &tauri::AppHandle,
) {
    let Some(querier_peer_id) = local_querier_peer_id(key_pair).await else {
        return;
    };

    let snapshot = lan_devices.lock().await.clone();
    if snapshot.is_empty() {
        return;
    }

    let probe_ports = app_profile::tcp_probe_port_candidates(tcp_port);
    let connected_peer_ids: HashSet<String> = connections
        .lock()
        .await
        .connected_peer_ids()
        .into_iter()
        .collect();

    let probe_results = futures_util::future::join_all(snapshot.iter().map(|device| {
        let peer_id = device.peer_id.clone();
        let ip = device.ip.clone();
        let port = device.port;
        let skip_probe = connected_peer_ids.contains(&peer_id);
        let probe_ports = probe_ports.clone();
        let querier_peer_id = querier_peer_id.clone();
        async move {
            if skip_probe {
                return (peer_id, true, None);
            }
            match direct::probe_planarclip_presence_resilient(
                &ip,
                port,
                &probe_ports,
                &querier_peer_id,
                Some(&peer_id),
                LAN_PROBE_TIMEOUT,
            )
            .await
            {
                Some(presence) => (peer_id, true, Some(presence)),
                None => (peer_id, false, None),
            }
        }
    }))
    .await;

    let unreachable: HashSet<_> = probe_results
        .iter()
        .filter(|(_, reachable, _)| !reachable)
        .map(|(peer_id, _, _)| peer_id.clone())
        .collect();

    let presence_updates: HashMap<_, _> = probe_results
        .into_iter()
        .filter_map(|(peer_id, reachable, presence)| {
            if reachable {
                presence.map(|result| (peer_id, result))
            } else {
                None
            }
        })
        .collect();

    let mut devices = lan_devices.lock().await;
    let mut changed = false;

    for device in devices.iter_mut() {
        if let Some(presence) = presence_updates.get(&device.peer_id) {
            apply_presence_probe(device, presence);
            changed = true;
        }
    }

    if !unreachable.is_empty() {
        let before = devices.len();
        devices.retain(|device| !unreachable.contains(&device.peer_id));
        if devices.len() != before {
            tracing::info!(
                "Pruned {} unreachable LAN device(s) after presence probe",
                before - devices.len()
            );
            changed = true;
        }
    }

    if !changed {
        return;
    }

    let updated = devices.clone();
    drop(devices);
    let _ = app.emit("lan-devices-changed", &updated);
}

pub(crate) async fn remove_lan_device_by_peer_id(
    lan_devices: &Arc<Mutex<Vec<LanDevice>>>,
    peer_id: &str,
    app: &tauri::AppHandle,
) {
    if peer_id.trim().is_empty() {
        return;
    }

    let mut devices = lan_devices.lock().await;
    let before = devices.len();
    devices.retain(|device| device.peer_id != peer_id);
    if devices.len() == before {
        return;
    }

    tracing::info!("Removed LAN device entry for offline peer {}", peer_id);
    let updated = devices.clone();
    drop(devices);
    let _ = app.emit("lan-devices-changed", &updated);
}

fn is_default_device_name(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case("My Device") || trimmed == DEFAULT_DEVICE_NAME
}

fn validate_device_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if is_default_device_name(trimmed) {
        return Ok(default_device_name());
    }

    if trimmed.chars().count() > 24 {
        return Err("设备名称最多 24 个字符，请缩短后再试。".to_string());
    }

    Ok(trimmed.to_string())
}

fn ui_settings_from_config(config: &AppConfig) -> UiSettingsPayload {
    UiSettingsPayload {
        color_scheme: config
            .ui_color_scheme
            .as_deref()
            .and_then(normalized_color_scheme)
            .unwrap_or(DEFAULT_UI_COLOR_SCHEME)
            .to_string(),
        theme_color: config
            .ui_theme_color
            .as_deref()
            .and_then(normalized_theme_color)
            .unwrap_or(DEFAULT_UI_THEME_COLOR)
            .to_string(),
        device_name: normalize_stored_device_name(&config.device_name),
    }
}

fn startup_settings_from_config(config: &AppConfig) -> StartupSettingsPayload {
    StartupSettingsPayload {
        launch_at_startup: config.launch_at_startup.unwrap_or(false),
        silent_start: config.silent_start.unwrap_or(false),
    }
}

fn connection_settings_from_config(config: &AppConfig) -> ConnectionSettingsPayload {
    ConnectionSettingsPayload {
        auto_connect_trusted: config.auto_connect_trusted.unwrap_or(true),
    }
}

fn max_file_bytes_to_mb(bytes: u64) -> u32 {
    ((bytes + 1024 * 1024 - 1) / (1024 * 1024)).max(1) as u32
}

fn max_file_mb_to_bytes(mb: u32) -> u64 {
    mb as u64 * 1024 * 1024
}

fn validate_max_file_mb(mb: u32) -> Result<u32, String> {
    if (1..=500).contains(&mb) {
        Ok(mb)
    } else {
        Err("文件大小上限无效，请输入 1 到 500 之间的整数。".to_string())
    }
}

fn sync_settings_from_config(config: &AppConfig) -> SyncSettingsPayload {
    let custom = config.sync_files_save_dir.as_deref();
    let is_default = custom.map(|value| value.trim().is_empty()).unwrap_or(true);
    let save_dir = storage::sync_save::resolve_sync_files_save_dir(if is_default {
        None
    } else {
        custom
    })
    .unwrap_or_else(|_| PathBuf::from(""));
    SyncSettingsPayload {
        sync_images: config.sync_images.unwrap_or(true),
        sync_files: config.sync_files.unwrap_or(true),
        max_file_mb: max_file_bytes_to_mb(
            config
                .max_file_bytes
                .unwrap_or(DEFAULT_MAX_FILE_BYTES),
        ),
        sync_files_save_enabled: config.sync_files_save_enabled.unwrap_or(false),
        sync_files_save_dir: save_dir.to_string_lossy().into_owned(),
        sync_files_save_dir_is_default: is_default,
        auto_sync_clipboard: config.auto_sync_clipboard.unwrap_or(true),
    }
}

fn normalized_clipboard_history_limit(value: Option<usize>) -> usize {
    match value {
        Some(limit) if CLIPBOARD_HISTORY_LIMIT_OPTIONS.contains(&limit) => limit,
        _ => DEFAULT_CLIPBOARD_HISTORY_LIMIT,
    }
}

fn validate_clipboard_history_limit(value: usize) -> Result<usize, String> {
    if CLIPBOARD_HISTORY_LIMIT_OPTIONS.contains(&value) {
        Ok(value)
    } else {
        Err("展示条数无效，请重新选择后再试。".to_string())
    }
}

fn normalized_clipboard_view_mode(value: Option<&str>) -> &'static str {
    match value {
        Some("list") => "list",
        _ => DEFAULT_CLIPBOARD_VIEW_MODE,
    }
}

fn validate_clipboard_view_mode(value: &str) -> Result<String, String> {
    match value {
        "list" => Ok("list".to_string()),
        "grid" => Ok("grid".to_string()),
        _ => Err("视图模式无效，请重新选择后再试。".to_string()),
    }
}

fn clipboard_settings_from_config(config: &AppConfig) -> ClipboardSettingsPayload {
    ClipboardSettingsPayload {
        history_limit: normalized_clipboard_history_limit(config.clipboard_history_limit),
        view_mode: normalized_clipboard_view_mode(config.clipboard_view_mode.as_deref()).to_string(),
    }
}

fn load_clipboard_history_from_config(config: &AppConfig) -> Vec<ClipboardHistoryEntry> {
    let limit = normalized_clipboard_history_limit(config.clipboard_history_limit);
    storage::history::load_history().into_iter().take(limit).collect()
}

pub(crate) async fn is_peer_connected(
    connections: &Arc<Mutex<ConnectionRegistry>>,
    peer_id: &str,
) -> bool {
    connections.lock().await.contains(peer_id)
}

async fn persist_clipboard_history(
    _config: &Arc<Mutex<AppConfig>>,
    history: &[ClipboardHistoryEntry],
) {
    storage::history::save_history(history);
}

fn sync_autostart(app: &tauri::AppHandle, launch_at_startup: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    let enabled = autostart
        .is_enabled()
        .map_err(|error| format!("读取开机启动状态失败：{}", error))?;

    if launch_at_startup && !enabled {
        autostart
            .enable()
            .map_err(|error| format!("开启开机启动失败：{}", error))?;
    } else if !launch_at_startup && enabled {
        autostart
            .disable()
            .map_err(|error| format!("关闭开机启动失败：{}", error))?;
    }

    Ok(())
}

fn trusted_peer_payload(peer: &TrustedPeerData) -> TrustedPeerPayload {
    TrustedPeerPayload {
        name: peer.name.clone(),
        peer_id: peer.peer_id.clone(),
        last_ip: peer.last_ip.clone(),
        auto_accept: peer.auto_accept.unwrap_or(true),
    }
}

const PAIRING_CODE_TTL_MS: i64 = 60_000;

fn pairing_code_expires_at_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
        + PAIRING_CODE_TTL_MS
}

async fn start_pairing_session(state: &AppState, code: Arc<Mutex<String>>) {
    *state.pairing_session_code.lock().await = Some(code);
    *state.pairing_code_expires_at.lock().await = Some(pairing_code_expires_at_ms());
}

async fn clear_pending_responder_submit(state: &AppState) {
    *state.pending_responder_submit_tx.lock().await = None;
}

async fn clear_pairing_session(state: &AppState) {
    *state.pairing_session_code.lock().await = None;
    *state.pairing_code_expires_at.lock().await = None;
    clear_pending_responder_submit(state).await;
}

async fn publish_initiator_local_pairing_code(
    state: &AppState,
    code: String,
) -> Result<(), direct::HandshakeError> {
    let mut stream = state
        .pending_initiator
        .lock()
        .await
        .take()
        .ok_or(direct::HandshakeError::Cancelled)?;

    let publish_result = direct::initiator_publish_local_code(&mut stream, code).await;
    *state.pending_initiator.lock().await = Some(stream);
    publish_result
}

async fn rotate_active_pairing_code(
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    let code_arc = state
        .pairing_session_code
        .lock()
        .await
        .clone()
        .ok_or_else(|| "当前没有进行中的配对验证。".to_string())?;

    let new_code = direct::generate_pairing_code();
    *code_arc.lock().await = new_code.clone();
    let expires_at_ms = pairing_code_expires_at_ms();
    *state.pairing_code_expires_at.lock().await = Some(expires_at_ms);

    if state.pending_initiator.lock().await.is_some() {
        publish_initiator_local_pairing_code(state, new_code.clone())
            .await
            .map_err(|error| error.user_message())?;
    }

    emit_pairing_code_rotated(app, &new_code, expires_at_ms);
    Ok(new_code)
}

async fn refresh_active_pairing_code(
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    if state.pairing_session_code.lock().await.is_some() {
        return rotate_active_pairing_code(state, app).await;
    }

    let new_code = direct::generate_pairing_code();
    let pairing_code = Arc::new(Mutex::new(new_code.clone()));
    start_pairing_session(state, pairing_code).await;
    let expires_at_ms = pairing_code_expires_at_ms();
    emit_pairing_code_rotated(app, &new_code, expires_at_ms);
    Ok(new_code)
}

fn emit_pairing_code_rotated(app: &tauri::AppHandle, code: &str, expires_at_ms: i64) {
    let _ = app.emit(
        "pairing-code-rotated",
        serde_json::json!({
            "code": code,
            "expires_at_ms": expires_at_ms,
        }),
    );
}

fn pairing_code_from_key_pair(kp: &KeyPair) -> String {
    direct::pairing_code_from_public_key_bytes(&kp.public_bytes())
}

fn peer_auto_accepts(peer: &TrustedPeerData) -> bool {
    peer.auto_accept.unwrap_or(true)
}

pub(crate) fn upsert_trusted_peer(config: &mut AppConfig, incoming: TrustedPeerData) -> bool {
    let incoming = TrustedPeerData {
        peer_id: peer_id_from_public_key(&incoming.public_key),
        ..incoming
    };
    let peers = config.trusted_peers.get_or_insert_with(Vec::new);
    if let Some(existing) = peers
        .iter_mut()
        .find(|peer| peer.public_key == incoming.public_key || peer.peer_id == incoming.peer_id)
    {
        let mut changed = false;
        if existing.name != incoming.name {
            existing.name = incoming.name;
            changed = true;
        }
        if existing.public_key != incoming.public_key {
            existing.public_key = incoming.public_key;
            changed = true;
        }
        if existing.peer_id != incoming.peer_id {
            existing.peer_id = incoming.peer_id;
            changed = true;
        }
        if incoming.last_ip.is_some() && existing.last_ip != incoming.last_ip {
            existing.last_ip = incoming.last_ip;
            changed = true;
        }
        return changed;
    }

    peers.push(incoming);
    true
}

/// Migrates trusted-device records to the current peer id and display-name rules.
fn normalize_trusted_peers(config: &mut AppConfig) -> bool {
    let Some(peers) = config.trusted_peers.as_mut() else {
        return false;
    };

    let mut changed = false;
    for peer in peers {
        let normalized_peer_id = peer_id_from_public_key(&peer.public_key);
        if peer.peer_id != normalized_peer_id {
            peer.peer_id = normalized_peer_id;
            changed = true;
        }
        if is_default_device_name(&peer.name) {
            peer.name = default_device_name();
            changed = true;
        }
    }
    changed
}

fn build_clipboard_history_entry(event: &ClipboardEvent) -> Option<ClipboardHistoryEntry> {
    let hash = hex::encode(event.content_hash());
    let (source_label, direction) = match &event.origin {
        ClipboardOrigin::Local => ("这台设备".to_string(), "sent".to_string()),
        ClipboardOrigin::Remote { peer_name } => {
            let label = if peer_name.trim().is_empty() {
                "已连接设备".to_string()
            } else {
                peer_name.clone()
            };
            (label, "received".to_string())
        }
    };

    match &event.snapshot {
        clipboard::types::ClipboardSnapshot::Text(text) => {
            let content = text.trim().to_string();
            if content.is_empty() {
                return None;
            }

            Some(ClipboardHistoryEntry {
                id: format!("{}-{}", event.timestamp_ms, &hash[..8]),
                content,
                clip_type: "text".to_string(),
                source_label,
                direction,
                timestamp_ms: event.timestamp_ms,
                size_label: None,
                image_data_url: None,
                image_ref: None,
                media_ref: None,
                file_count: None,
                file_names: None,
                preview_kind: None,
                thumbnail_ref: None,
            })
        }
        clipboard::types::ClipboardSnapshot::Image {
            png_bytes,
            width,
            height,
        } => {
            let id = format!("{}-{}", event.timestamp_ms, &hash[..8]);
            let media_ref = match storage::history_media::write_media(&id, png_bytes) {
                Ok(reference) => Some(reference),
                Err(error) => {
                    tracing::warn!("failed to persist history image: {error}");
                    None
                }
            };
            Some(ClipboardHistoryEntry {
                id,
                content: format!("[图片] {width}×{height}"),
                clip_type: "image".to_string(),
                source_label,
                direction,
                timestamp_ms: event.timestamp_ms,
                size_label: Some(clipboard::image::format_byte_size(png_bytes.len())),
                image_data_url: None,
                image_ref: None,
                media_ref,
                file_count: None,
                file_names: None,
                preview_kind: None,
                thumbnail_ref: None,
            })
        }
        clipboard::types::ClipboardSnapshot::FileList { files } => {
            let is_single_image =
                files.len() == 1 && clipboard::file::is_image_file_name(&files[0].file_name);
            let id = format!("{}-{}", event.timestamp_ms, &hash[..8]);
            let media_ref = if is_single_image {
                clipboard::file::history_preview_for_files(files, &id)
            } else {
                None
            };
            Some(ClipboardHistoryEntry {
                id,
                content: clipboard::file::history_summary_for_files(files),
                clip_type: if is_single_image {
                    "image".to_string()
                } else {
                    "file".to_string()
                },
                source_label,
                direction,
                timestamp_ms: event.timestamp_ms,
                size_label: Some(clipboard::file::file_list_size_label(files)),
                image_data_url: None,
                image_ref: None,
                media_ref,
                file_count: if is_single_image {
                    None
                } else {
                    Some(files.len() as u32)
                },
                file_names: if is_single_image {
                    None
                } else {
                    Some(clipboard::file::file_names_for_history(files))
                },
                preview_kind: None,
                thumbnail_ref: None,
            })
        }
        clipboard::types::ClipboardSnapshot::Empty => None,
    }
}

fn merge_clipboard_history(
    history: &mut Vec<ClipboardHistoryEntry>,
    entry: ClipboardHistoryEntry,
    limit: usize,
) {
    if history.first().map(|item| item.id.as_str()) == Some(entry.id.as_str()) {
        return;
    }

    if entry.clip_type == "text"
        && history.first().map(|item| item.content.as_str()) == Some(entry.content.as_str())
        && history.first().map(|item| item.direction.as_str()) == Some(entry.direction.as_str())
        && history.first().map(|item| item.source_label.as_str()) == Some(entry.source_label.as_str())
    {
        return;
    }

    if let Some(first) = history.first_mut() {
        let is_recent_opposite_duplicate = first.content == entry.content
            && first.direction != entry.direction
            && first.timestamp_ms.abs_diff(entry.timestamp_ms) <= 2_000;

        if is_recent_opposite_duplicate {
            if entry.direction == "received" {
                *first = entry;
            }
            return;
        }
    }

    history.insert(0, entry);
    if history.len() > limit {
        let removed: Vec<ClipboardHistoryEntry> = history.drain(limit..).collect();
        for item in removed {
            if let Some(media_ref) = item.media_ref {
                let _ = storage::history_media::delete_by_ref(&media_ref);
            }
        }
    }
}

fn snapshot_from_history_entry(entry: &ClipboardHistoryEntry) -> Result<clipboard::types::ClipboardSnapshot, String> {
    match entry.clip_type.as_str() {
        "text" => {
            let content = entry.content.trim();
            if content.is_empty() {
                return Err("该条历史内容为空，无法操作。".to_string());
            }
            Ok(clipboard::types::ClipboardSnapshot::Text(content.to_string()))
        }
        "image" => {
            let data_url = entry
                .image_data_url
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "无法读取该图片的历史数据，请重新复制后再试。".to_string())?;
            let png_bytes = clipboard::image::png_bytes_from_data_url(data_url)?;
            clipboard::image::snapshot_from_png_bytes(png_bytes)
                .ok_or_else(|| "该图片过大或格式无效，无法操作。".to_string())
        }
        "file" => Err("文件类历史暂不支持从这里重新写入剪贴板，请在资源管理器中重新复制文件。".to_string()),
        _ => Err("不支持的历史类型。".to_string()),
    }
}

fn write_history_snapshot_to_clipboard(snapshot: &clipboard::types::ClipboardSnapshot) {
    match snapshot {
        clipboard::types::ClipboardSnapshot::Text(text) => {
            ClipboardMonitor::write_clipboard(text);
        }
        clipboard::types::ClipboardSnapshot::Image {
            png_bytes,
            width,
            height,
        } => ClipboardMonitor::write_clipboard_image(png_bytes, *width, *height),
        clipboard::types::ClipboardSnapshot::FileList { .. } | clipboard::types::ClipboardSnapshot::Empty => {}
    }
}

fn fresh_promoted_history_id(entry: &ClipboardHistoryEntry, timestamp_ms: u64) -> String {
    let hash = hex::encode(
        blake3::hash(format!("promote:{}:{}", entry.id, timestamp_ms).as_bytes()).as_bytes(),
    );
    format!("{}-{}", timestamp_ms, &hash[..8])
}

fn promote_history_entry_clone(
    history: &mut Vec<ClipboardHistoryEntry>,
    entry_id: &str,
    limit: usize,
) -> Result<(), String> {
    if history.first().map(|item| item.id.as_str()) == Some(entry_id) {
        return Ok(());
    }

    let index = history
        .iter()
        .position(|item| item.id == entry_id)
        .ok_or_else(|| "找不到这条剪贴板历史，可能已被清空。".to_string())?;

    let mut promoted = history[index].clone();
    promoted.timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(promoted.timestamp_ms);
    promoted.id = fresh_promoted_history_id(&promoted, promoted.timestamp_ms);
    history.insert(0, promoted);

    if history.len() > limit {
        let removed: Vec<ClipboardHistoryEntry> = history.drain(limit..).collect();
        for item in removed {
            if let Some(media_ref) = item.media_ref {
                let _ = storage::history_media::delete_by_ref(&media_ref);
            }
        }
    }

    Ok(())
}

fn single_file_preview_source(
    event: &ClipboardEvent,
    entry: &ClipboardHistoryEntry,
) -> Option<(std::path::PathBuf, String, u64, String)> {
    if entry.clip_type != "file" || entry.file_count != Some(1) {
        return None;
    }

    let clipboard::types::ClipboardSnapshot::FileList { files } = &event.snapshot else {
        return None;
    };
    let file = files.first()?;
    let path = file.source_path.clone()?;

    Some((
        path,
        file.file_name.clone(),
        file.size_bytes,
        entry.id.clone(),
    ))
}

async fn active_connection_count(state: &AppState) -> usize {
    state.connections.lock().await.len()
}

fn next_connection_generation(state: &AppState) -> u64 {
    state.connection_generation.fetch_add(1, Ordering::SeqCst) + 1
}

async fn is_duplicate_active_session(
    connections: &Arc<Mutex<ConnectionRegistry>>,
    peer_id: &str,
) -> bool {
    is_peer_connected(connections, peer_id).await
}

async fn abort_outbound_handshake(state: &AppState) {
    if let Some(tx) = state.outbound_abort.lock().await.take() {
        let _ = tx.send(());
    }
    if let Some(stream) = state.pending_outbound.lock().await.take() {
        direct::initiator_abort(stream).await;
    }
}

async fn clear_pending_connection_request(state: &AppState) {
    *state.pending_connection_request.lock().await = None;
}

pub(crate) fn emit_outbound_connection_started(
    app: &tauri::AppHandle,
    peer_id: &str,
    peer_name: &str,
    ip: &str,
    port: u16,
    source: &str,
) {
    let _ = app.emit(
        "outbound-connection-started",
        serde_json::json!({
            "peer_id": peer_id,
            "peer_name": peer_name,
            "peer_ip": ip,
            "peer_port": port,
            "source": source,
        }),
    );
}

pub(crate) fn emit_outbound_connection_settled(app: &tauri::AppHandle, peer_id: &str) {
    let _ = app.emit(
        "outbound-connection-settled",
        serde_json::json!({
            "peer_id": peer_id,
        }),
    );
}

pub(crate) fn emit_connection_failed(app: &tauri::AppHandle, error: &direct::HandshakeError) {
    let _ = app.emit(
        "connection-failed",
        serde_json::json!({
            "kind": error.reason_code(),
            "message": error.user_message(),
        }),
    );
}

async fn watch_pending_initiator_peer(
    pending_initiator: Arc<Mutex<Option<TcpStream>>>,
    app: tauri::AppHandle,
    state: AppState,
) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let stream = {
            let mut guard = pending_initiator.lock().await;
            guard.take()
        };

        let Some(stream) = stream else {
            return;
        };

        match direct::poll_initiator_peer_message(stream).await {
            Ok((direct::InitiatorPendingMessage::None, Some(stream))) => {
                *pending_initiator.lock().await = Some(stream);
            }
            Ok((direct::InitiatorPendingMessage::PeerFailed(error), _)) => {
                let error = if matches!(
                    error.reason_code(),
                    "connection_lost" | "connection_unavailable" | "rejected"
                ) {
                    direct::HandshakeError::PeerCancelled
                } else {
                    error
                };
                emit_connection_failed(&app, &error);
                return;
            }
            Ok((direct::InitiatorPendingMessage::PeerAccepted(conn), _)) => {
                finalize_outbound_pairing(&state, &app, conn, None).await;
                return;
            }
            Ok((_, None)) => return,
            Err(error) => {
                let error = if matches!(
                    error.reason_code(),
                    "connection_lost" | "connection_unavailable"
                ) {
                    direct::HandshakeError::PeerCancelled
                } else {
                    error
                };
                emit_connection_failed(&app, &error);
                return;
            }
        }
    }
}

async fn finalize_outbound_pairing(
    state: &AppState,
    app: &tauri::AppHandle,
    conn: direct::DirectConnection,
    last_ip: Option<String>,
) {
    let peer_name = conn.peer_name.clone();
    let peer_id = conn.peer_id.clone();
    let peer_pk = conn.peer_public_key.clone();

    if is_duplicate_active_session(&state.connections, &peer_id).await {
        tracing::info!(
            "Skipping duplicate outbound pairing session with already-connected peer {}",
            peer_name
        );
        return;
    }

    {
        let mut config = state.config.lock().await;
        upsert_trusted_peer(
            &mut config,
            TrustedPeerData {
                name: peer_name.clone(),
                public_key: peer_pk,
                peer_id: peer_id.clone(),
                last_ip,
                auto_accept: None,
            },
        );
        storage_json::save_config(&config);
    }

    let session_generation = next_connection_generation(state);
    ConnectionManager::connect_direct(
        conn,
        state.connections.clone(),
        state.connection_generation.clone(),
        session_generation,
        state.clip_tx.clone(),
        app.clone(),
    )
    .await;

    let _ = app.emit(
        "connection-established",
        serde_json::json!({
            "peer_name": peer_name,
            "peer_id": peer_id,
            "is_reconnect": false,
        }),
    );
    window::send_session_established_notification(app, &peer_name, false);
    clear_pairing_session(state).await;
}

pub(crate) async fn store_pending_initiator_stream(
    pending_initiator: Arc<Mutex<Option<TcpStream>>>,
    app: &tauri::AppHandle,
    state: AppState,
    stream: TcpStream,
) {
    *pending_initiator.lock().await = Some(stream);
    let pending = pending_initiator.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        watch_pending_initiator_peer(pending, app, state).await;
    });
}

#[tauri::command]
async fn get_pending_connection_request(
    state: tauri::State<'_, AppState>,
) -> Result<Option<window::ConnectionRequestPayload>, String> {
    Ok(state.pending_connection_request.lock().await.clone())
}

#[tauri::command]
async fn get_connected_peer(
    state: tauri::State<'_, AppState>,
) -> Result<Option<ConnectedPeerPayload>, String> {
    Ok(state.connections.lock().await.connected_peers().into_iter().next())
}

#[tauri::command]
async fn get_connected_peers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ConnectedPeerPayload>, String> {
    Ok(state.connections.lock().await.connected_peers())
}

#[tauri::command]
async fn get_startup_settings(
    state: tauri::State<'_, AppState>,
) -> Result<StartupSettingsPayload, String> {
    let config = state.config.lock().await;
    Ok(startup_settings_from_config(&config))
}

#[tauri::command]
async fn save_startup_settings(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    launch_at_startup: bool,
    silent_start: bool,
) -> Result<StartupSettingsPayload, String> {
    {
        let mut config = state.config.lock().await;
        config.launch_at_startup = Some(launch_at_startup);
        config.silent_start = Some(silent_start);
        storage_json::save_config(&config);
    }

    sync_autostart(&app, launch_at_startup)?;

    let config = state.config.lock().await;
    Ok(startup_settings_from_config(&config))
}

#[tauri::command]
async fn get_app_behavior_settings(
    state: tauri::State<'_, AppState>,
) -> Result<AppBehaviorSettingsPayload, String> {
    let config = state.config.lock().await;
    Ok(app_behavior_from_config(&config))
}

#[tauri::command]
async fn save_app_behavior_settings(
    state: tauri::State<'_, AppState>,
    system_notifications_enabled: bool,
    close_window_action: String,
) -> Result<AppBehaviorSettingsPayload, String> {
    let action = normalized_close_window_action(Some(close_window_action.as_str()));
    {
        let mut config = state.config.lock().await;
        config.system_notifications_enabled = Some(system_notifications_enabled);
        config.close_window_action = Some(action.to_string());
        storage_json::save_config(&config);
    }

    let config = state.config.lock().await;
    Ok(app_behavior_from_config(&config))
}

#[tauri::command]
async fn get_connection_settings(
    state: tauri::State<'_, AppState>,
) -> Result<ConnectionSettingsPayload, String> {
    let config = state.config.lock().await;
    Ok(connection_settings_from_config(&config))
}

#[tauri::command]
async fn save_connection_settings(
    state: tauri::State<'_, AppState>,
    auto_connect_trusted: bool,
) -> Result<ConnectionSettingsPayload, String> {
    {
        let mut config = state.config.lock().await;
        config.auto_connect_trusted = Some(auto_connect_trusted);
        storage_json::save_config(&config);
    }

    let config = state.config.lock().await;
    Ok(connection_settings_from_config(&config))
}

#[tauri::command]
async fn get_sync_settings(state: tauri::State<'_, AppState>) -> Result<SyncSettingsPayload, String> {
    let config = state.config.lock().await;
    Ok(sync_settings_from_config(&config))
}

#[tauri::command]
async fn save_sync_settings(
    state: tauri::State<'_, AppState>,
    sync_images: bool,
    sync_files: Option<bool>,
    max_file_mb: Option<u32>,
    sync_files_save_enabled: Option<bool>,
    auto_sync_clipboard: Option<bool>,
) -> Result<SyncSettingsPayload, String> {
    {
        let mut config = state.config.lock().await;
        config.sync_images = Some(sync_images);
        if let Some(sync_files) = sync_files {
            config.sync_files = Some(sync_files);
        }
        if let Some(sync_files_save_enabled) = sync_files_save_enabled {
            config.sync_files_save_enabled = Some(sync_files_save_enabled);
        }
        if let Some(auto_sync_clipboard) = auto_sync_clipboard {
            config.auto_sync_clipboard = Some(auto_sync_clipboard);
        }
        if let Some(max_file_mb) = max_file_mb {
            config.max_file_bytes = Some(max_file_mb_to_bytes(validate_max_file_mb(max_file_mb)?));
        } else if config.max_file_bytes.is_none() {
            config.max_file_bytes = Some(DEFAULT_MAX_FILE_BYTES);
        }
        storage_json::save_config(&config);
    }

    let config = state.config.lock().await;
    Ok(sync_settings_from_config(&config))
}

#[tauri::command]
async fn pick_sync_files_save_dir() -> Result<String, String> {
    let picked = tokio::task::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|_| "打开文件夹选择窗口失败，请稍后再试。".to_string())?;
    picked
        .map(|path| path.to_string_lossy().into_owned())
        .ok_or_else(|| "未选择文件夹。".to_string())
}

#[tauri::command]
async fn save_sync_files_save_dir(
    state: tauri::State<'_, AppState>,
    path: Option<String>,
) -> Result<SyncSettingsPayload, String> {
    {
        let mut config = state.config.lock().await;
        config.sync_files_save_dir = match path {
            Some(value) if value.trim().is_empty() => None,
            Some(value) => {
                storage::sync_save::resolve_sync_files_save_dir(Some(value.trim()))?;
                Some(value.trim().to_string())
            }
            None => None,
        };
        storage_json::save_config(&config);
    }

    let config = state.config.lock().await;
    Ok(sync_settings_from_config(&config))
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, AppState>) -> Result<String, String> {
    if state.connections.lock().await.is_empty() {
        Ok("disconnected".into())
    } else {
        Ok("connected".into())
    }
}

#[tauri::command]
async fn get_pairing_code(state: tauri::State<'_, AppState>) -> Result<String, String> {
    if let Some(code_arc) = state.pairing_session_code.lock().await.as_ref() {
        return Ok(code_arc.lock().await.clone());
    }

    let kp_guard = state.key_pair.lock().await;
    if let Some(ref kp) = *kp_guard {
        Ok(pairing_code_from_key_pair(kp))
    } else {
        Err("密钥对尚未初始化".into())
    }
}

#[tauri::command]
async fn rotate_pairing_code(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    refresh_active_pairing_code(state.inner(), &app).await
}

#[tauri::command]
async fn end_pairing_session(state: tauri::State<'_, AppState>) -> Result<(), String> {
    clear_pairing_session(state.inner()).await;
    Ok(())
}

#[tauri::command]
async fn abort_outbound_connection(state: tauri::State<'_, AppState>) -> Result<(), String> {
    abort_outbound_handshake(state.inner()).await;
    if let Some(stream) = state.pending_initiator.lock().await.take() {
        direct::initiator_abort(stream).await;
    }
    clear_pairing_session(state.inner()).await;
    Ok(())
}

#[tauri::command]
async fn get_clipboard_settings(
    state: tauri::State<'_, AppState>,
) -> Result<ClipboardSettingsPayload, String> {
    let config = state.config.lock().await;
    Ok(clipboard_settings_from_config(&config))
}

#[tauri::command]
async fn save_clipboard_settings(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    history_limit: Option<usize>,
    view_mode: Option<String>,
) -> Result<ClipboardSettingsPayload, String> {
    if history_limit.is_none() && view_mode.is_none() {
        return Err("没有可保存的剪贴板设置。".to_string());
    }

    let validated_limit = history_limit.map(validate_clipboard_history_limit).transpose()?;
    let validated_view_mode = view_mode
        .as_deref()
        .map(validate_clipboard_view_mode)
        .transpose()?;

    {
        let mut config = state.config.lock().await;
        if let Some(limit) = validated_limit {
            config.clipboard_history_limit = Some(limit);
        }
        if let Some(mode) = validated_view_mode {
            config.clipboard_view_mode = Some(mode);
        }
        storage_json::save_config(&config);
    }

    if let Some(limit) = validated_limit {
        let updated_history = {
            let mut history = state.clipboard_history.lock().await;
            history.truncate(limit);
            history.clone()
        };
        persist_clipboard_history(&state.config, &updated_history).await;
        let _ = app.emit("clipboard-history-changed", updated_history);
    }

    let config = state.config.lock().await;
    Ok(clipboard_settings_from_config(&config))
}

#[tauri::command]
async fn clear_clipboard_history(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut history = state.clipboard_history.lock().await;
        history.clear();
    }

    let (sync_files, max_file_bytes) = {
        let config = state.config.lock().await;
        (
            config.sync_files.unwrap_or(true),
            config
                .max_file_bytes
                .unwrap_or(DEFAULT_MAX_FILE_BYTES),
        )
    };
    *state.clipboard_dedup_baseline.lock().await = Some(ClipboardMonitor::capture_dedup_baseline(
        max_file_bytes,
        sync_files,
    ));

    state
        .clipboard_monitor_generation
        .fetch_add(1, Ordering::SeqCst);

    let updated_history = Vec::new();
    persist_clipboard_history(&state.config, &updated_history).await;
    let _ = storage::history_media::clear_all();
    let _ = app.emit("clipboard-history-changed", updated_history);
    Ok(())
}

#[tauri::command]
fn read_history_media(media_ref: String) -> Result<String, String> {
    storage::history_media::resolve_data_url(&media_ref)
}

/// Resolve a file-type icon by extension, cached per-extension under `cache/icons/`
/// so identical types reuse one icon file (never stored per history entry).
#[tauri::command]
fn read_type_icon(ext: String) -> Result<String, String> {
    let parent = storage::json::config_path()
        .parent()
        .ok_or("data dir unavailable")?
        .to_path_buf();
    let cache_dir = parent.join("cache").join("icons");
    let safe_ext: String = ext
        .trim_start_matches('.')
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if safe_ext.is_empty() {
        return Err("invalid extension".into());
    }
    let cache_path = cache_dir.join(format!("{safe_ext}.png"));
    if cache_path.is_file() {
        let bytes = std::fs::read(&cache_path).map_err(|e| format!("read type icon: {e}"))?;
        return Ok(data_url_png(&bytes));
    }
    let png = generate_type_icon(&safe_ext).ok_or_else(|| "type icon unavailable".to_string())?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("create type icon dir: {e}"))?;
    std::fs::write(&cache_path, &png).map_err(|e| format!("write type icon: {e}"))?;
    Ok(data_url_png(&png))
}

fn data_url_png(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    format!("data:image/png;base64,{}", BASE64.encode(bytes))
}

#[cfg(windows)]
fn generate_type_icon(ext: &str) -> Option<Vec<u8>> {
    platform::windows::thumbnail::shell_file_icon_by_ext(ext, 32)
}

#[cfg(not(windows))]
fn generate_type_icon(_ext: &str) -> Option<Vec<u8>> {
    None
}

/// Change the TCP listen port. If the new port is free, stop the old broadcast
/// (if any) and start a new one; if not free, stop broadcasting and report conflict.
#[tauri::command]
async fn set_tcp_port(
    port: u16,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BroadcastState, String> {
    if port < 1024 || port == 1420 || port == 1421 {
        return Err("端口无效（需 1024–65535，且不能是 1420/1421）".to_string());
    }
    {
        let mut config = state.config.lock().await;
        config.tcp_port = Some(port);
        storage_json::save_config(&config);
    }

    // Stop any existing broadcast before (re)starting on the new port.
    if let Some(old) = state.broadcast_handles.lock().await.take() {
        stop_broadcast(old);
    }

    let new_state = if direct::is_port_available(port) {
        match start_broadcast(&app, port, true, true).await {
            Ok(handles) => {
                *state.broadcast_handles.lock().await = Some(handles);
                BroadcastState::Active { port }
            }
            Err(e) => {
                tracing::error!("start_broadcast failed: {e}");
                BroadcastState::Inactive { port, reason: e }
            }
        }
    } else {
        tracing::warn!("port {} occupied, browse-only", port);
        match start_broadcast(&app, port, false, false).await {
            Ok(handles) => {
                *state.broadcast_handles.lock().await = Some(handles);
            }
            Err(e) => {
                tracing::error!("browse-only start failed: {e}");
            }
        }
        BroadcastState::PortConflict { port }
    };

    *state.broadcast_state.lock().await = new_state.clone();
    let _ = app.emit("broadcast-state-changed", &new_state);
    Ok(new_state)
}

#[tauri::command]
async fn get_broadcast_state(state: tauri::State<'_, AppState>) -> Result<BroadcastState, String> {
    Ok(state.broadcast_state.lock().await.clone())
}

#[tauri::command]
async fn get_ui_settings(state: tauri::State<'_, AppState>) -> Result<UiSettingsPayload, String> {
    let config = state.config.lock().await;
    Ok(ui_settings_from_config(&config))
}

#[tauri::command]
async fn get_shell_bootstrap(state: tauri::State<'_, AppState>) -> Result<ShellBootstrapPayload, String> {
    let config = state.config.lock().await;
    let ui_settings = ui_settings_from_config(&config);
    drop(config);

    let status = if state.connections.lock().await.is_empty() {
        "disconnected".to_string()
    } else {
        "connected".to_string()
    };

    let pairing_code = if let Some(code_arc) = state.pairing_session_code.lock().await.as_ref() {
        code_arc.lock().await.clone()
    } else {
        let kp_guard = state.key_pair.lock().await;
        if let Some(ref kp) = *kp_guard {
            pairing_code_from_key_pair(kp)
        } else {
            return Err("密钥对尚未初始化".into());
        }
    };

    let connected_peers = state.connections.lock().await.connected_peers();
    let pending_connection_request = state.pending_connection_request.lock().await.clone();
    let broadcast_state = state.broadcast_state.lock().await.clone();

    Ok(ShellBootstrapPayload {
        ui_settings,
        status,
        pairing_code,
        connected_peers,
        pending_connection_request,
        broadcast_state,
    })
}

#[tauri::command]
async fn get_shell_deferred(state: tauri::State<'_, AppState>) -> Result<ShellDeferredPayload, String> {
    let (clipboard_settings, auto_sync_clipboard) = {
        let config = state.config.lock().await;
        (
            clipboard_settings_from_config(&config),
            config.auto_sync_clipboard.unwrap_or(true),
        )
    };

    let mut trusted_peers = {
        let config = state.config.lock().await;
        config
            .trusted_peers
            .clone()
            .unwrap_or_default()
            .iter()
            .map(trusted_peer_payload)
            .collect::<Vec<_>>()
    };
    trusted_peers.sort_by(|left, right| left.name.cmp(&right.name));

    let clipboard_history = state.clipboard_history.lock().await.clone();
    let lan_devices = state.lan_devices.lock().await.clone();

    Ok(ShellDeferredPayload {
        clipboard_history,
        lan_devices,
        trusted_peers,
        clipboard_settings,
        auto_sync_clipboard,
    })
}

#[tauri::command]
fn notify_main_ui_ready(state: tauri::State<'_, AppState>, app: tauri::AppHandle) {
    state
        .main_window_ui_ready
        .store(true, std::sync::atomic::Ordering::Release);
    window::try_reveal_pending_main_window(&app, false);
}

#[tauri::command]
async fn get_clipboard_history(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ClipboardHistoryEntry>, String> {
    let history = state.clipboard_history.lock().await.clone();
    Ok(history)
}

#[tauri::command]
async fn copy_clipboard_history_entry(
    state: tauri::State<'_, AppState>,
    entry_id: String,
) -> Result<(), String> {
    let entry = {
        let history = state.clipboard_history.lock().await;
        history
            .iter()
            .find(|item| item.id == entry_id)
            .cloned()
            .ok_or_else(|| "找不到这条剪贴板历史，可能已被清空。".to_string())?
    };

    let snapshot = snapshot_from_history_entry(&entry)?;
    write_history_snapshot_to_clipboard(&snapshot);
    Ok(())
}

#[tauri::command]
async fn send_clipboard_history_entry(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    entry_id: String,
) -> Result<(), String> {
    let (entry, index) = {
        let history = state.clipboard_history.lock().await;
        let index = history
            .iter()
            .position(|item| item.id == entry_id)
            .ok_or_else(|| "找不到这条剪贴板历史，可能已被清空。".to_string())?;
        (history[index].clone(), index)
    };

    let snapshot = if entry.clip_type == "file" {
        let (sync_files, max_file_bytes) = {
            let config = state.config.lock().await;
            (
                config.sync_files.unwrap_or(true),
                config
                    .max_file_bytes
                    .unwrap_or(DEFAULT_MAX_FILE_BYTES),
            )
        };
        if !sync_files {
            return Err("文件同步已关闭，请先在设置中开启同步文件。".to_string());
        }
        let snapshot =
            ClipboardMonitor::read_current_snapshot(max_file_bytes, sync_files)?;
        match &snapshot {
            clipboard::types::ClipboardSnapshot::FileList { files } if !files.is_empty() => {
                snapshot
            }
            _ => {
                return Err(
                    "剪贴板里没有可同步的文件，请在资源管理器中重新复制后再发送。".to_string(),
                );
            }
        }
    } else {
        let snapshot = snapshot_from_history_entry(&entry)?;
        write_history_snapshot_to_clipboard(&snapshot);
        snapshot
    };

    if index > 0 {
        let updated_history = {
            let limit = {
                let cfg = state.config.lock().await;
                normalized_clipboard_history_limit(cfg.clipboard_history_limit)
            };
            let mut history = state.clipboard_history.lock().await;
            promote_history_entry_clone(&mut history, &entry_id, limit)?;
            history.clone()
        };
        persist_clipboard_history(&state.config, &updated_history).await;
        let _ = app.emit("clipboard-history-changed", updated_history);
    }

    let _ = state
        .clip_tx
        .send(ClipboardEvent::local_sync_only(snapshot));

    Ok(())
}

#[tauri::command]
async fn save_ui_settings(
    state: tauri::State<'_, AppState>,
    color_scheme: String,
    theme_color: String,
    device_name: String,
) -> Result<UiSettingsPayload, String> {
    let color_scheme = normalized_color_scheme(&color_scheme)
        .ok_or_else(|| "当前背景模式无效，请重新选择后再试。".to_string())?;
    let theme_color = normalized_theme_color(&theme_color)
        .ok_or_else(|| "当前主题色无效，请重新选择后再试。".to_string())?;
    let device_name = validate_device_name(&device_name)?;

    let mut config = state.config.lock().await;
    config.ui_color_scheme = Some(color_scheme.to_string());
    config.ui_theme_color = Some(theme_color.to_string());
    config.device_name = device_name;
    storage_json::save_config(&config);

    Ok(ui_settings_from_config(&config))
}

#[tauri::command]
async fn pair(state: tauri::State<'_, AppState>, code: String) -> Result<String, String> {
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("配对码必须为 6 位数字".into());
    }

    let peer_id = {
        let kp = state.key_pair.lock().await;
        match *kp {
            Some(ref kp) => kp.fingerprint(),
            None => return Err("密钥对尚未初始化".into()),
        }
    };

    tracing::info!("Pairing: connecting to room {} as peer {}", code, peer_id);

    let handle = ConnectionManager::connect(
        SIGNALLING_SERVER,
        &code,
        &peer_id,
        Arc::new(Mutex::new(true)),
        state.clip_tx.clone(),
    )
    .await
    .map_err(|e| format!("连接失败：{}", e))?;

    {
        let mut config = state.config.lock().await;
        config.paired_peer = Some(PeerData {
            name: "已配对设备".into(),
            public_key: vec![],
        });
        storage_json::save_config(&config);
    }

    let session_generation = next_connection_generation(state.inner());
    state.connections.lock().await.insert(
        peer_id.clone(),
        network::sessions::ConnectionSession {
            handle,
            peer_name: "已配对设备".into(),
            session_generation,
            connected: Arc::new(Mutex::new(true)),
        },
    );

    Ok("paired".into())
}

#[tauri::command]
async fn get_lan_devices(state: tauri::State<'_, AppState>) -> Result<Vec<LanDevice>, String> {
    let devices = state.lan_devices.lock().await.clone();
    Ok(devices)
}

#[tauri::command]
async fn refresh_lan_devices(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<LanDevice>, String> {
    let tcp_port = state.config.lock().await.tcp_port.unwrap_or(app_profile::DEFAULT_TCP_PORT);
    refresh_lan_presence(
        &state.config,
        &state.key_pair,
        &state.lan_devices,
        &state.connections,
        &state.peer_offline_cooldown,
        tcp_port,
        &app,
    )
    .await;
    let devices = state.lan_devices.lock().await.clone();
    Ok(devices)
}

#[tauri::command]
async fn get_trusted_peers(state: tauri::State<'_, AppState>) -> Result<Vec<TrustedPeerPayload>, String> {
    let config = state.config.lock().await;
    let mut peers = config
        .trusted_peers
        .clone()
        .unwrap_or_default()
        .iter()
        .map(trusted_peer_payload)
        .collect::<Vec<_>>();
    peers.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(peers)
}

#[tauri::command]
async fn remove_trusted_peer(state: tauri::State<'_, AppState>, peer_id: String) -> Result<bool, String> {
    let peer_id = peer_id.trim();
    if peer_id.is_empty() {
        return Err("缺少要解除信任的设备标识。".to_string());
    }

    let mut config = state.config.lock().await;
    let Some(peers) = config.trusted_peers.as_mut() else {
        return Ok(false);
    };

    let before_len = peers.len();
    peers.retain(|peer| peer.peer_id != peer_id);
    let removed = peers.len() != before_len;
    if removed {
        storage_json::save_config(&config);
    }

    Ok(removed)
}

#[tauri::command]
async fn set_peer_auto_accept(
    state: tauri::State<'_, AppState>,
    peer_id: String,
    auto_accept: bool,
) -> Result<bool, String> {
    let peer_id = peer_id.trim();
    if peer_id.is_empty() {
        return Err("缺少设备标识。".to_string());
    }

    let mut config = state.config.lock().await;
    let Some(peers) = config.trusted_peers.as_mut() else {
        return Ok(false);
    };

    let Some(peer) = peers.iter_mut().find(|peer| peer.peer_id == peer_id) else {
        return Ok(false);
    };

    let next = Some(auto_accept);
    if peer.auto_accept == next {
        return Ok(true);
    }

    peer.auto_accept = next;
    storage_json::save_config(&config);
    Ok(true)
}

#[tauri::command]
async fn connect_lan(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    ip: String,
    port: u16,
    peer_id: Option<String>,
) -> Result<String, String> {
    if active_connection_count(state.inner()).await >= MAX_CONNECTIONS {
        return Err("已超出连接上限".into());
    }

    let (device_name, key_pair, requires_confirmation) = {
        let config = state.config.lock().await;
        let kp = state.key_pair.lock().await;
        let kp = kp.clone().ok_or("密钥对尚未初始化")?;
        let target_peer_id = peer_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let initiator_knows_peer = target_peer_id.as_deref().is_some_and(|target| {
            config
                .trusted_peers
                .as_deref()
                .unwrap_or_default()
                .iter()
                .any(|peer| peer.peer_id == target)
        });
        (
            config.device_name.clone(),
            kp,
            !initiator_knows_peer,
        )
    };

    let handshake = match direct::initiator_send_connect_request(
        &ip,
        port,
        &device_name,
        &key_pair,
        requires_confirmation,
    )
    .await
    {
        Ok(stream) => {
            let outbound = Arc::new(Mutex::new(Some(stream)));
            let (abort_tx, abort_rx) = oneshot::channel();
            *state.outbound_abort.lock().await = Some(abort_tx);

            let outbound_read = outbound.clone();
            let read_task = async move {
                let mut guard = outbound_read.lock().await;
                let stream = guard.take().ok_or(direct::HandshakeError::Cancelled)?;
                direct::initiator_read_connect_response(stream).await
            };

            let result = tokio::select! {
                result = read_task => result,
                _ = abort_rx => {
                    let mut guard = outbound.lock().await;
                    if let Some(stream) = guard.take() {
                        direct::initiator_abort(stream).await;
                    }
                    Err(direct::HandshakeError::Cancelled)
                }
            };

            *state.outbound_abort.lock().await = None;
            result
        }
        Err(error) => Err(error),
    };

    match handshake {
        Ok(InitiatorResult::Connected(conn)) => {
            let peer_name = conn.peer_name.clone();
            let peer_id = conn.peer_id.clone();
            let peer_pk = conn.peer_public_key.clone();

            {
                let mut config = state.config.lock().await;
                upsert_trusted_peer(
                    &mut config,
                    TrustedPeerData {
                        name: peer_name.clone(),
                        public_key: peer_pk,
                        peer_id: peer_id.clone(),
                        last_ip: Some(ip),
                        auto_accept: None,
                    },
                );
                storage_json::save_config(&config);
            }

            let session_generation = next_connection_generation(state.inner());
            ConnectionManager::connect_direct(
                conn,
                state.connections.clone(),
                state.connection_generation.clone(),
                session_generation,
                state.clip_tx.clone(),
                app.clone(),
            )
            .await;

            let _ = app.emit(
                "connection-established",
                serde_json::json!({
                    "peer_name": peer_name,
                    "peer_id": peer_id,
                    "is_reconnect": true,
                }),
            );
            window::send_session_established_notification(&app, &peer_name, true);

            Ok("connected".into())
        }
        Ok(InitiatorResult::AwaitingCode { mut stream }) => {
            let session_code = direct::generate_pairing_code();
            let pairing_code = Arc::new(Mutex::new(session_code.clone()));
            start_pairing_session(state.inner(), pairing_code).await;
            let expires_at_ms = pairing_code_expires_at_ms();
            emit_pairing_code_rotated(&app, &session_code, expires_at_ms);

            if let Err(error) = direct::initiator_publish_local_code(&mut stream, session_code).await
            {
                clear_pairing_session(state.inner()).await;
                emit_connection_failed(&app, &error);
                return Err(error.user_message());
            }

            store_pending_initiator_stream(
                state.pending_initiator.clone(),
                &app,
                state.inner().clone(),
                stream,
            )
            .await;
            let _ = app.emit(
                "pairing-code-needed",
                serde_json::json!({
                    "peer_ip": ip,
                }),
            );
            Ok("awaiting_code".into())
        }
        Err(e) => {
            emit_connection_failed(&app, &e);
            Err(e.user_message())
        }
    }
}

#[tauri::command]
async fn submit_pairing_code(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    code: String,
) -> Result<String, String> {
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("配对码必须为 6 位数字".into());
    }

    let stream = {
        let mut guard = state.pending_initiator.lock().await;
        guard.take()
    };
    let stream = match stream {
        Some(s) => s,
        None => {
            // watch_pending_initiator_peer briefly takes the stream every 300ms
            // to poll (50ms window); retry once after it puts the stream back.
            tracing::warn!("submit_pairing_code: pending_initiator None, retrying");
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            state
                .pending_initiator
                .lock()
                .await
                .take()
                .ok_or("当前没有待处理的连接")?
        }
    };

    match direct::initiator_send_code(stream, code).await {
        Ok(conn) => {
            finalize_outbound_pairing(state.inner(), &app, conn, None).await;
            Ok("connected".into())
        }
        Err(e) => {
            emit_connection_failed(&app, &e);
            Err(e.user_message())
        }
    }
}

#[tauri::command]
async fn submit_responder_pairing_code(
    state: tauri::State<'_, AppState>,
    code: String,
) -> Result<String, String> {
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("配对码必须为 6 位数字".into());
    }

    let submit_tx = state
        .pending_responder_submit_tx
        .lock()
        .await
        .clone()
        .ok_or("当前没有待处理的入站配对")?;

    let (reply_tx, reply_rx) = oneshot::channel();
    submit_tx
        .send((code, reply_tx))
        .await
        .map_err(|_| "配对流程已结束，请重新发起连接".to_string())?;

    match reply_rx.await {
        Ok(Ok(())) => Ok("verified".into()),
        Ok(Err(error)) => Err(error.user_message()),
        Err(_) => Err("配对流程已结束，请重新发起连接".into()),
    }
}

#[tauri::command]
async fn accept_connection(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.pending_accept_tx.lock().await.take();
    let _ = state.pending_incoming_timeout_tx.lock().await.take();
    if let Some(tx) = tx {
        let _ = tx.send(());
        clear_pending_connection_request(state.inner()).await;
        Ok(())
    } else {
        Err("对方已取消这次连接".into())
    }
}

#[tauri::command]
async fn reject_connection(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let _ = state.pending_accept_tx.lock().await.take();
    let _ = state.pending_incoming_timeout_tx.lock().await.take();
    if let Some(tx) = state.pending_reject_tx.lock().await.take() {
        let _ = tx.send(()).await;
    }
    clear_pending_connection_request(state.inner()).await;
    spawn_lan_presence_refresh(state.inner(), &app);
    Ok(())
}

#[tauri::command]
async fn timeout_incoming_connection(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let _ = state.pending_accept_tx.lock().await.take();
    let _ = state.pending_reject_tx.lock().await.take();
    if let Some(tx) = state.pending_incoming_timeout_tx.lock().await.take() {
        let _ = tx.send(()).await;
    }
    clear_pending_connection_request(state.inner()).await;
    Ok(())
}

#[tauri::command]
async fn disconnect(state: tauri::State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    let disconnected_peers = state.connections.lock().await.connected_peers();

    next_connection_generation(state.inner());
    abort_outbound_handshake(state.inner()).await;

    let local_peer_id = {
        let key_pair = state.key_pair.lock().await;
        key_pair
            .as_ref()
            .map(|pair| peer_id_from_public_key(pair.public.as_bytes()))
            .unwrap_or_default()
    };

    let sessions = state.connections.lock().await.drain_all();
    let had_sessions = !sessions.is_empty();
    for (_, session) in sessions {
        session.handle.notify_peer_left(&local_peer_id);
    }
    if had_sessions {
        tokio::time::sleep(Duration::from_millis(120)).await;
    }

    if let Some(stream) = state.pending_initiator.lock().await.take() {
        direct::initiator_abort(stream).await;
    }

    let _ = state.pending_accept_tx.lock().await.take();
    let _ = state.pending_incoming_timeout_tx.lock().await.take();
    if let Some(tx) = state.pending_reject_tx.lock().await.take() {
        let _ = tx.send(()).await;
    }

    clear_pairing_session(&state).await;

    spawn_lan_presence_refresh(state.inner(), &app);

    if disconnected_peers.len() == 1 {
        let peer_name = disconnected_peers[0].peer_name();
        let trimmed = peer_name.trim();
        let message = if trimmed.is_empty() {
            "设备 已断开连接".to_string()
        } else {
            format!("{trimmed} 已断开连接")
        };
        window::send_session_ended_notification(&app, &message);
    } else if !disconnected_peers.is_empty() {
        window::send_session_ended_notification(&app, "已断开所有连接");
    }

    Ok(())
}

#[tauri::command]
async fn disconnect_peer(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    peer_id: String,
) -> Result<(), String> {
    let peer_id = peer_id.trim();
    if peer_id.is_empty() {
        return Err("缺少设备标识。".into());
    }

    let session = state.connections.lock().await.remove(peer_id);
    let Some(session) = session else {
        return Err("未找到该设备的连接。".into());
    };

    let local_peer_id = {
        let key_pair = state.key_pair.lock().await;
        key_pair
            .as_ref()
            .map(|pair| peer_id_from_public_key(pair.public.as_bytes()))
            .unwrap_or_default()
    };

    session.handle.notify_peer_left(&local_peer_id);
    tokio::time::sleep(Duration::from_millis(120)).await;
    *session.connected.lock().await = false;

    spawn_lan_presence_refresh(state.inner(), &app);

    let peer_name = session.peer_name.trim();
    let message = if peer_name.is_empty() {
        "设备 已断开连接".to_string()
    } else {
        format!("{peer_name} 已断开连接")
    };
    window::send_session_ended_notification(&app, &message);
    Ok(())
}

fn load_or_create_key_pair(config: &mut AppConfig) -> KeyPair {
    if let Some(ref stored) = config.key_pair {
        if stored.secret_bytes.len() == 32 {
            let mut secret_arr = [0u8; 32];
            secret_arr.copy_from_slice(&stored.secret_bytes);
            let secret = x25519_dalek::StaticSecret::from(secret_arr);
            let public = x25519_dalek::PublicKey::from(&secret);
            tracing::info!("Restored key pair from config");
            return KeyPair { secret, public };
        }
    }

    let kp = KeyPair::generate();
    config.key_pair = Some(KeyPairData {
        secret_bytes: kp.secret.as_bytes().to_vec(),
        public_bytes: kp.public_bytes().to_vec(),
    });
    tracing::info!("Generated new key pair");
    kp
}

/// Resolve the active instance directory before logging initializes.
///
/// Order: reuse an existing `<peer_id>/` dir -> migrate legacy flat config ->
/// create a fresh identity. Returns the instance paths, the key pair, and the
/// loaded (or default) config so `run()` can continue without re-loading.
fn bootstrap_instance() -> (crate::storage::paths::InstancePaths, KeyPair, AppConfig) {
    use crate::storage::paths;

    let _ = std::fs::create_dir_all(paths::data_root());

    // Existing install directory with an id -> use it.
    if let Some(id) = paths::read_install_id() {
        let instance = paths::InstancePaths::new(&id);
        let _ = instance.ensure_dir();
        let mut config = storage_json::load_config_at(&instance.config_path());
        let key_pair = load_or_create_key_pair(&mut config);
        eprintln!("[bootstrap] using instance {id}");
        return (instance, key_pair, config);
    }

    // First launch for this install directory: generate a new identity.
    let mut config = AppConfig::default();
    let key_pair = load_or_create_key_pair(&mut config);
    let id = key_pair.fingerprint();
    paths::write_install_id(&id);
    let instance = paths::InstancePaths::new(&id);
    let _ = instance.ensure_dir();
    storage_json::save_config_at(&config, &instance.config_path());
    eprintln!("[bootstrap] created instance {id}");
    (instance, key_pair, config)
}

/// Migrate legacy inline base64 and legacy `history_thumbs/`/`history_images/` refs
/// into unified `history_media/` files with `media_ref`. Type-icon thumbnails are
/// dropped (the frontend renders type icons now).
fn migrate_history_media(config: &mut AppConfig) {
    if let Some(parent) = storage::json::config_path().parent() {
        let moved = storage::history_media::migrate_legacy_dir(parent);
        if moved > 0 {
            tracing::info!("migrated {moved} legacy media files into history_media/");
        }
    }

    let mut migrated = 0;
    if let Some(history) = config.clipboard_history.as_mut() {
        migrated += migrate_media_refs(history);
    }

    let mut history = storage::history::load_history();
    if !history.is_empty() {
        let n = migrate_media_refs(&mut history);
        if n > 0 {
            storage::history::save_history(&history);
            migrated += n;
        }
    }

    if migrated > 0 {
        tracing::info!("migrated {migrated} history media refs");
    }
}

fn migrate_media_refs(history: &mut [ClipboardHistoryEntry]) -> usize {
    let mut count = 0;
    for entry in history.iter_mut() {
        if let Some(data_url) = entry.image_data_url.take() {
            if let Ok(png_bytes) = clipboard::image::png_bytes_from_data_url(&data_url) {
                if let Ok(reference) = storage::history_media::write_media(&entry.id, &png_bytes) {
                    entry.media_ref = Some(reference);
                    count += 1;
                }
            }
        }
        if entry.media_ref.is_none() {
            if let Some(image_ref) = entry.image_ref.take() {
                entry.media_ref = Some(rewrite_media_ref(&image_ref));
                count += 1;
            }
        }
        if entry.media_ref.is_none() && entry.preview_kind.as_deref() != Some("icon") {
            if let Some(thumbnail_ref) = entry.thumbnail_ref.take() {
                entry.media_ref = Some(rewrite_media_ref(&thumbnail_ref));
                count += 1;
            }
        }
        entry.thumbnail_ref = None;
    }
    count
}

fn rewrite_media_ref(legacy: &str) -> String {
    let name = legacy.rsplit('/').next().unwrap_or(legacy);
    format!("history_media/{name}")
}

/// Move `clipboard_history` out of config into a standalone `history.json`,
/// so config carries only settings (no user content).
fn migrate_history_to_file(config: &mut AppConfig) {
    if let Some(history) = config.clipboard_history.take() {
        storage::history::save_history(&history);
        tracing::info!("migrated clipboard history to history.json ({} entries)", history.len());
    }
}

async fn finalize_incoming_connection(
    conn: network::direct::DirectConnection,
    initiator_name: String,
    initiator_peer_id: String,
    initiator_pk: Vec<u8>,
    initiator_ip: Option<String>,
    is_reconnect: bool,
    config: Arc<Mutex<AppConfig>>,
    connections: Arc<Mutex<ConnectionRegistry>>,
    connection_generation: Arc<AtomicU64>,
    clip_tx: broadcast::Sender<ClipboardEvent>,
    app_handle: &tauri::AppHandle,
) {
    if is_duplicate_active_session(&connections, &initiator_peer_id).await {
        tracing::info!(
            "Skipping duplicate incoming connection from already-connected peer {}",
            initiator_name
        );
        return;
    }

    {
        let mut cfg = config.lock().await;
        upsert_trusted_peer(
            &mut cfg,
            TrustedPeerData {
                name: initiator_name.clone(),
                public_key: initiator_pk.clone(),
                peer_id: initiator_peer_id.clone(),
                last_ip: initiator_ip.clone(),
                auto_accept: None,
            },
        );
        storage_json::save_config(&cfg);
    }

    let session_generation = next_connection_generation(
        app_handle.state::<AppState>().inner(),
    );
    ConnectionManager::connect_direct(
        conn,
        connections.clone(),
        connection_generation,
        session_generation,
        clip_tx.clone(),
        app_handle.clone(),
    )
    .await;

    let _ = app_handle.emit(
        "connection-established",
        serde_json::json!({
            "peer_name": initiator_name,
            "peer_id": initiator_peer_id,
            "is_reconnect": is_reconnect,
        }),
    );
    window::send_session_established_notification(app_handle, &initiator_name, is_reconnect);
}

async fn handle_incoming_connection(
    req: direct::IncomingRequest,
    key_pair: KeyPair,
    config: Arc<Mutex<AppConfig>>,
    app_handle: tauri::AppHandle,
    connections: Arc<Mutex<ConnectionRegistry>>,
    connection_generation: Arc<AtomicU64>,
    clip_tx: broadcast::Sender<ClipboardEvent>,
    pending_accept_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pending_reject_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
) {
    let initiator_name = req.initiator_name.clone();
    let initiator_peer_id = req.initiator_peer_id.clone();
    let initiator_pk = req.initiator_public_key.clone();
    let initiator_ip = req.stream.peer_addr().ok().map(|addr| addr.ip().to_string());

    // 每次从配置读取设备名称、熟悉关系与自动接受策略，避免移除或改名后必须重启。
    let (device_name, initiator_is_familiar, should_auto_accept) = {
        let cfg = config.lock().await;
        let trusted_peer = cfg
            .trusted_peers
            .clone()
            .unwrap_or_default()
            .into_iter()
            .find(|tp| tp.public_key == initiator_pk);
        let initiator_is_familiar = trusted_peer.is_some();
        let should_auto_accept = trusted_peer
            .as_ref()
            .map(|peer| peer_auto_accepts(peer))
            .unwrap_or(false);
        (normalize_stored_device_name(&cfg.device_name), initiator_is_familiar, should_auto_accept)
    };

    tracing::debug!(
        "Incoming connection from {} (familiar={}, initiator_requires_confirmation={})",
        initiator_name,
        initiator_is_familiar,
        req.requires_confirmation,
    );

    if should_auto_accept {
        tracing::info!("Auto-accepting trusted peer: {}", initiator_name);
        match direct::responder_accept_trusted(
            req.stream,
            &device_name,
            &key_pair,
            initiator_name.clone(),
            initiator_pk.clone(),
        )
        .await
        {
            Ok(conn) => {
                finalize_incoming_connection(
                    conn,
                    initiator_name,
                    initiator_peer_id,
                    initiator_pk,
                    initiator_ip,
                    true,
                    config,
                    connections.clone(),
                    app_handle.state::<AppState>().connection_generation.clone(),
                    clip_tx,
                    &app_handle,
                )
                .await;
            }
            Err(e) => {
                tracing::warn!("Auto-accept handshake failed: {}", e);
            }
        }
        return;
    }

    let pending_connection_request = app_handle.state::<AppState>().pending_connection_request.clone();

    if initiator_is_familiar {
        tracing::info!(
            "Familiar peer {} — waiting for user confirmation",
            initiator_name,
        );

        let (accept_tx, accept_rx) = oneshot::channel();
        let (reject_tx, reject_rx) = mpsc::channel(1);
        let (timeout_tx, timeout_rx) = mpsc::channel(1);
        *pending_accept_tx.lock().await = Some(accept_tx);
        *pending_reject_tx.lock().await = Some(reject_tx);
        *app_handle
            .state::<AppState>()
            .pending_incoming_timeout_tx
            .lock()
            .await = Some(timeout_tx);

        window::present_connection_request(
            &app_handle,
            &pending_connection_request,
            initiator_name.clone(),
            initiator_peer_id.clone(),
            false,
        )
        .await;

        match direct::responder_wait_for_decision(
            req.stream,
            &device_name,
            &key_pair,
            initiator_name.clone(),
            initiator_pk.clone(),
            accept_rx,
            reject_rx,
            timeout_rx,
        )
        .await
        {
            Ok(conn) => {
                finalize_incoming_connection(
                    conn,
                    initiator_name,
                    initiator_peer_id,
                    initiator_pk,
                    initiator_ip,
                    true,
                    config,
                    connections.clone(),
                    app_handle.state::<AppState>().connection_generation.clone(),
                    clip_tx,
                    &app_handle,
                )
                .await;
            }
            Err(e) => {
                tracing::info!("Incoming confirmation failed: {}", e);
                *pending_accept_tx.lock().await = None;
                *pending_reject_tx.lock().await = None;
                *app_handle
                    .state::<AppState>()
                    .pending_incoming_timeout_tx
                    .lock()
                    .await = None;
                clear_pending_connection_request(app_handle.state::<AppState>().inner()).await;
                if !matches!(
                    e,
                    direct::HandshakeError::Cancelled | direct::HandshakeError::Timeout
                ) {
                    emit_connection_failed(&app_handle, &e);
                }
            }
        }
    } else {
        let session_code = direct::generate_pairing_code();
        let pairing_code = Arc::new(Mutex::new(session_code));
        {
            let app_state = app_handle.state::<AppState>();
            start_pairing_session(app_state.inner(), pairing_code.clone()).await;
            let initial_code = pairing_code.lock().await.clone();
            let expires_at_ms = pairing_code_expires_at_ms();
            emit_pairing_code_rotated(&app_handle, &initial_code, expires_at_ms);
        }

        tracing::info!(
            "Unfamiliar peer {} — waiting for pairing code",
            initiator_name,
        );

        let (accept_tx, accept_rx) = oneshot::channel();
        let (reject_tx, reject_rx) = mpsc::channel(2);
        let (timeout_tx, timeout_rx) = mpsc::channel(1);
        let (submit_tx, submit_rx) = mpsc::channel(4);
        *pending_accept_tx.lock().await = Some(accept_tx);
        *pending_reject_tx.lock().await = Some(reject_tx);
        *app_handle
            .state::<AppState>()
            .pending_incoming_timeout_tx
            .lock()
            .await = Some(timeout_tx);
        *app_handle
            .state::<AppState>()
            .pending_responder_submit_tx
            .lock()
            .await = Some(submit_tx);

        window::present_connection_request(
            &app_handle,
            &pending_connection_request,
            initiator_name.clone(),
            initiator_peer_id.clone(),
            true,
        )
        .await;

        let app_handle_rotate = app_handle.clone();

        match direct::responder_verify_code(
            req.stream,
            &device_name,
            &key_pair,
            initiator_name.clone(),
            initiator_pk.clone(),
            pairing_code,
            move |new_code| {
                let app = app_handle_rotate.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    let expires_at_ms = pairing_code_expires_at_ms();
                    *state.pairing_code_expires_at.lock().await = Some(expires_at_ms);
                    emit_pairing_code_rotated(&app, &new_code, expires_at_ms);
                });
            },
            accept_rx,
            reject_rx,
            timeout_rx,
            submit_rx,
        )
        .await
        {
            Ok(conn) => {
                finalize_incoming_connection(
                    conn,
                    initiator_name,
                    initiator_peer_id,
                    initiator_pk,
                    initiator_ip,
                    false,
                    config,
                    connections.clone(),
                    app_handle.state::<AppState>().connection_generation.clone(),
                    clip_tx,
                    &app_handle,
                )
                .await;
            }
            Err(e) => {
                tracing::info!("Incoming pairing failed: {}", e);
                *pending_accept_tx.lock().await = None;
                *pending_reject_tx.lock().await = None;
                *app_handle
                    .state::<AppState>()
                    .pending_incoming_timeout_tx
                    .lock()
                    .await = None;
                clear_pending_connection_request(app_handle.state::<AppState>().inner()).await;
                if !matches!(
                    e,
                    direct::HandshakeError::Cancelled
                        | direct::HandshakeError::InvalidCode
                        | direct::HandshakeError::Timeout
                ) {
                    emit_connection_failed(&app_handle, &e);
                }
            }
        }
    }

    {
        let app_state = app_handle.state::<AppState>();
        clear_pairing_session(app_state.inner()).await;
        clear_pending_connection_request(app_state.inner()).await;
    }

    *pending_reject_tx.lock().await = None;
    *pending_accept_tx.lock().await = None;
    *app_handle
        .state::<AppState>()
        .pending_incoming_timeout_tx
        .lock()
        .await = None;
}

#[tauri::command]
async fn get_diagnostic_settings(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(state.config.lock().await.verbose_log.unwrap_or(false))
}

#[tauri::command]
async fn save_diagnostic_settings(
    state: tauri::State<'_, AppState>,
    verbose_log: bool,
) -> Result<bool, String> {
    {
        let mut config = state.config.lock().await;
        config.verbose_log = Some(verbose_log);
        storage_json::save_config(&config);
    }
    logging::set_verbose(verbose_log);
    tracing::info!(target: "logging", verbose = verbose_log, "diagnostic verbosity updated");
    Ok(verbose_log)
}

pub fn run() {
    // Resolve the instance directory (migrating legacy data on first run of the
    // new layout) before logging so log_dir/staging/thumbs target the instance.
    let (instance, key_pair, mut config) = bootstrap_instance();
    crate::storage::paths::set_instance(instance);

    // File-rotated, non-blocking logging; guard flushes the writer on process exit.
    let _log_guard = logging::init();
    tracing::info!(
        "instance dir: {}",
        crate::storage::paths::current_config_path().display()
    );

    match storage::staging::gc_staging() {
        Ok(removed) if removed > 0 => tracing::info!("staging GC removed {removed} expired item(s)"),
        Err(error) => tracing::warn!("staging GC failed: {error}"),
        _ => {}
    }

    config.device_name = normalize_stored_device_name(&config.device_name);
    normalize_trusted_peers(&mut config);
    // Migrate legacy inline image base64 and legacy media dirs into history_media/.
    migrate_history_media(&mut config);
    // Move clipboard_history out of config into history.json (config stays small).
    migrate_history_to_file(&mut config);
    // key_pair already resolved in bootstrap_instance; persist normalized config.
    storage_json::save_config(&config);

    // GC orphan media files not referenced by history.
    let media_refs: Vec<String> = storage::history::load_history()
        .iter()
        .filter_map(|e| e.media_ref.clone())
        .collect();
    storage::history_media::gc_orphans(&media_refs);

    logging::set_verbose(config.verbose_log.unwrap_or(false));

    let silent_tray_startup = config.silent_start.unwrap_or(false);
    let mut context = tauri::generate_context!();
    if silent_tray_startup {
        for window in context.config_mut().app.windows.iter_mut() {
            window.create = false;
            window.visible = false;
            window.focus = false;
        }
    }

    let tcp_port = config.tcp_port.unwrap_or(app_profile::DEFAULT_TCP_PORT);
    let initial_clipboard_history = load_clipboard_history_from_config(&config);

    let (clip_tx, _) = broadcast::channel::<ClipboardEvent>(16);

    let app_state = AppState {
        config: Arc::new(Mutex::new(config)),
        key_pair: Arc::new(Mutex::new(Some(key_pair.clone()))),
        connections: Arc::new(Mutex::new(ConnectionRegistry::new())),
        transfer_slots: Arc::new(sync::transfer_limit::TransferSlotLimiter::new(
            sync::transfer_limit::MAX_CONCURRENT_TRANSFERS,
        )),
        connection_generation: Arc::new(AtomicU64::new(0)),
        clipboard_monitor_generation: Arc::new(AtomicU64::new(0)),
        clipboard_dedup_baseline: Arc::new(Mutex::new(None)),
        clip_tx: clip_tx.clone(),
        clipboard_history: Arc::new(Mutex::new(initial_clipboard_history)),
        lan_devices: Arc::new(Mutex::new(Vec::new())),
        peer_offline_cooldown: Arc::new(Mutex::new(HashMap::new())),
        pending_initiator: Arc::new(Mutex::new(None)),
        pending_outbound: Arc::new(Mutex::new(None)),
        outbound_abort: Arc::new(Mutex::new(None)),
        outbound_handshake_active: Arc::new(AtomicBool::new(false)),
        pending_accept_tx: Arc::new(Mutex::new(None)),
        pending_reject_tx: Arc::new(Mutex::new(None)),
        pending_incoming_timeout_tx: Arc::new(Mutex::new(None)),
        pending_connection_request: Arc::new(Mutex::new(None)),
        pairing_session_code: Arc::new(Mutex::new(None)),
        pairing_code_expires_at: Arc::new(Mutex::new(None)),
        pending_responder_submit_tx: Arc::new(Mutex::new(None)),
        main_window_ui_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        main_window_reveal_steal_focus: Arc::new(std::sync::Mutex::new(None)),
        broadcast_state: Arc::new(Mutex::new(BroadcastState::Inactive {
            port: tcp_port,
            reason: "starting".to_string(),
        })),
        broadcast_handles: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(APP_DISPLAY_NAME)
                .build(),
        )
        .manage(app_state)
        .setup(move |app| {
            use tauri_plugin_notification::NotificationExt;

            #[cfg(windows)]
            {
                let config = app.config();
                platform::windows::register_app_user_model_id(
                    &config.identifier,
                    config
                        .product_name
                        .as_deref()
                        .unwrap_or(APP_DISPLAY_NAME),
                );
            }

            let startup = startup_settings_from_config(&app.state::<AppState>().config.blocking_lock());

            let _ = app.handle().notification().request_permission();

            if let Err(error) = sync_autostart(app.handle(), startup.launch_at_startup) {
                tracing::warn!("Failed to sync autostart on startup: {}", error);
            }
            if !startup.silent_start {
                if let Err(error) = window::bootstrap_main_window(app.handle()) {
                    tracing::error!("Failed to create main window at startup: {}", error);
                }
            }

            let toggle = MenuItemBuilder::with_id("toggle", format!("打开 {APP_DISPLAY_NAME}")).build(app)?;
            let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&toggle, &separator, &quit])
                .build()?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(format!("{APP_DISPLAY_NAME}"))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        window::toggle_main_window(&tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle" => {
                        window::ensure_main_window(app.clone());
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            let clipboard_history = app.state::<AppState>().clipboard_history.clone();
            let config = app.state::<AppState>().config.clone();
            let connections = app.state::<AppState>().connections.clone();
            let transfer_slots = app.state::<AppState>().transfer_slots.clone();

            // Start mDNS + TCP listener if the configured port is free; otherwise
            // stay silent and prompt the user to pick another port.
            {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    if direct::is_port_available(tcp_port) {
                        match start_broadcast(&app_handle, tcp_port, true, true).await {
                            Ok(handles) => {
                                *state.broadcast_handles.lock().await = Some(handles);
                                *state.broadcast_state.lock().await =
                                    BroadcastState::Active { port: tcp_port };
                            }
                            Err(e) => {
                                tracing::error!("start_broadcast failed: {e}");
                                *state.broadcast_state.lock().await =
                                    BroadcastState::Inactive { port: tcp_port, reason: e };
                            }
                        }
                    } else {
                        tracing::warn!("port {} occupied, browse-only", tcp_port);
                        if let Ok(handles) =
                            start_broadcast(&app_handle, tcp_port, false, false).await
                        {
                            *state.broadcast_handles.lock().await = Some(handles);
                        }
                        *state.broadcast_state.lock().await =
                            BroadcastState::PortConflict { port: tcp_port };
                    }
                    let bs = state.broadcast_state.lock().await.clone();
                    let _ = app_handle.emit("broadcast-state-changed", &bs);
                });
            }


            {
                let app_handle = app_handle.clone();
                let clipboard_history = clipboard_history.clone();
                let config = config.clone();
                let mut clip_history_rx = clip_tx.subscribe();

                tauri::async_runtime::spawn(async move {
                    while let Ok(event) = clip_history_rx.recv().await {
                        if event.skip_history_merge {
                            continue;
                        }
                        if let Some(entry) = build_clipboard_history_entry(&event) {
                            let preview_source = single_file_preview_source(&event, &entry);
                            let updated_history = {
                                let limit = {
                                    let cfg = config.lock().await;
                                    normalized_clipboard_history_limit(cfg.clipboard_history_limit)
                                };
                                let mut history = clipboard_history.lock().await;
                                merge_clipboard_history(&mut history, entry, limit);
                                history.clone()
                            };
                            persist_clipboard_history(&config, &updated_history).await;
                            let _ = app_handle.emit("clipboard-history-changed", updated_history);

                            if let Some((path, file_name, size_bytes, entry_id)) = preview_source {
                                let config = config.clone();
                                let clipboard_history = clipboard_history.clone();
                                let app_handle = app_handle.clone();
                                let find_entry_id = entry_id.clone();
                                tauri::async_runtime::spawn(async move {
                                    let preview = tokio::task::spawn_blocking(move || {
                                        clipboard::history_preview::generate_and_store_preview(
                                            &path,
                                            &file_name,
                                            size_bytes,
                                            &entry_id,
                                        )
                                    })
                                    .await
                                    .ok()
                                    .flatten();

                                    let Some(preview) = preview else {
                                        return;
                                    };

                                    let updated_history = {
                                        let mut history = clipboard_history.lock().await;
                                        if let Some(item) =
                                            history.iter_mut().find(|item| item.id == find_entry_id)
                                        {
                                            item.preview_kind = Some(preview.kind.to_string());
                                            item.media_ref = preview.media_ref;
                                        }
                                        history.clone()
                                    };
                                    persist_clipboard_history(&config, &updated_history).await;
                                    let _ = app_handle.emit("clipboard-history-changed", updated_history);
                                });
                            }
                        }
                    }
                });
            }

            let clip_tx_monitor = clip_tx.clone();
            let clip_rx = clip_tx.subscribe();
            let connections_bg = connections.clone();
            let transfer_slots_bg = transfer_slots.clone();
            let config_bg = config.clone();
            let app_handle_bg = app_handle.clone();
            let clipboard_monitor_generation = app
                .state::<AppState>()
                .clipboard_monitor_generation
                .clone();
            let clipboard_dedup_baseline = app
                .state::<AppState>()
                .clipboard_dedup_baseline
                .clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create background runtime");
                rt.block_on(async move {
                    let config_monitor = config_bg.clone();
                    let app_handle_monitor = app_handle_bg.clone();
                    tokio::join!(
                        async move {
                            let mut monitor = ClipboardMonitor::new(
                                clip_tx_monitor,
                                config_monitor,
                                Some(app_handle_monitor),
                                clipboard_monitor_generation,
                                clipboard_dedup_baseline,
                            );
                            monitor.run().await;
                        },
                        async move {
                            let provider = std::sync::Arc::new(
                                sync::out::RegistryOutProvider::new(
                                    connections_bg.clone(),
                                    app_handle_bg.clone(),
                                ),
                            );
                            let engine = sync::engine::SyncEngine::new(
                                clip_rx,
                                provider,
                                transfer_slots_bg,
                                config_bg,
                            );
                            engine.run().await;
                        },
                    )
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_connected_peer,
            get_connected_peers,
            get_pairing_code,
            rotate_pairing_code,
            end_pairing_session,
            abort_outbound_connection,
            get_ui_settings,
            get_shell_bootstrap,
            get_shell_deferred,
            notify_main_ui_ready,
            get_startup_settings,
            save_startup_settings,
            get_app_behavior_settings,
            save_app_behavior_settings,
            get_connection_settings,
            save_connection_settings,
            get_sync_settings,
            save_sync_settings,
            pick_sync_files_save_dir,
            save_sync_files_save_dir,
            get_clipboard_settings,
            save_clipboard_settings,
            get_clipboard_history,
            copy_clipboard_history_entry,
            send_clipboard_history_entry,
            clear_clipboard_history,
            read_history_media,
            read_type_icon,
            set_tcp_port,
            get_broadcast_state,
            get_pending_connection_request,
            save_ui_settings,
            pair,
            get_lan_devices,
            refresh_lan_devices,
            get_trusted_peers,
            remove_trusted_peer,
            set_peer_auto_accept,
            connect_lan,
            submit_pairing_code,
            submit_responder_pairing_code,
            accept_connection,
            reject_connection,
            timeout_incoming_connection,
            disconnect,
            disconnect_peer,
            logging::frontend_log,
            logging::open_log_dir,
            get_diagnostic_settings,
            save_diagnostic_settings,
        ])
        .build(context)
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                // Keep tray-only mode alive when the last WebView is destroyed (silent start / close to tray).
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
