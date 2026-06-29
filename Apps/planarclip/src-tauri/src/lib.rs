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
use network::webrtc::{ConnectionHandle, ConnectionManager};
use storage::json::{self as storage_json, AppConfig, KeyPairData, PeerData, TrustedPeerData};

const SIGNALLING_SERVER: &str = "ws://localhost:8765";
const DEFAULT_DEVICE_NAME: &str = "我的设备";
const DEFAULT_UI_COLOR_SCHEME: &str = "dark";
const DEFAULT_UI_THEME_COLOR: &str = "cyan";
const CLIPBOARD_HISTORY_LIMIT_OPTIONS: [usize; 5] = [25, 50, 100, 200, 500];
const DEFAULT_CLIPBOARD_HISTORY_LIMIT: usize = 100;
const MAX_CONNECTIONS: usize = 5;
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
    pub connected: Arc<Mutex<bool>>,
    pub connected_peer: Arc<Mutex<Option<ConnectedPeerPayload>>>,
    pub connection: Arc<Mutex<Option<ConnectionHandle>>>,
    pub connection_generation: Arc<AtomicU64>,
    pub clipboard_monitor_generation: Arc<AtomicU64>,
    pub clipboard_dedup_baseline: Arc<Mutex<Option<ClipboardDedupBaseline>>>,
    pub clip_tx: broadcast::Sender<ClipboardEvent>,
    pub clipboard_history: Arc<Mutex<Vec<ClipboardHistoryEntry>>>,
    pub lan_devices: Arc<Mutex<Vec<LanDevice>>>,
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
struct ConnectionSettingsPayload {
    auto_connect_trusted: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
struct SyncSettingsPayload {
    sync_images: bool,
    sync_files: bool,
    max_file_mb: u32,
}

#[derive(Clone, Debug, serde::Serialize)]
struct ClipboardSettingsPayload {
    history_limit: usize,
}

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

    // Entries discovered before service_fullname was tracked still use display name.
    service_fullname.starts_with(&app_profile::mdns_service_fullname_prefix(&device.name))
}

const LAN_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const LAN_PROBE_INTERVAL: Duration = Duration::from_secs(8);
const LAN_PRESENCE_REFRESH_DELAY: Duration = Duration::from_millis(350);

async fn discover_trusted_peers_by_tcp_probe(
    config: &Arc<Mutex<AppConfig>>,
    lan_devices: &Arc<Mutex<Vec<LanDevice>>>,
    tcp_port: u16,
    app: &tauri::AppHandle,
) {
    let (trusted_peers, known_peer_ids) = {
        let config_guard = config.lock().await;
        let peers = config_guard.trusted_peers.clone().unwrap_or_default();
        let known_peer_ids = lan_devices
            .lock()
            .await
            .iter()
            .map(|device| device.peer_id.clone())
            .collect::<std::collections::HashSet<_>>();
        (peers, known_peer_ids)
    };

    let probe_ports = app_profile::tcp_probe_port_candidates(tcp_port);
    let mut discovered = Vec::new();

    for peer in trusted_peers {
        if known_peer_ids.contains(&peer.peer_id) {
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

        let Some(port) =
            direct::probe_tcp_reachable_on_any_port(last_ip, &probe_ports, LAN_PROBE_TIMEOUT).await
        else {
            continue;
        };

        discovered.push(LanDevice {
            name: peer.name.clone(),
            peer_id: peer.peer_id.clone(),
            ip: last_ip.to_string(),
            host_name: String::new(),
            port,
            service_fullname: String::new(),
        });
    }

    if discovered.is_empty() {
        return;
    }

    let mut devices = lan_devices.lock().await;
    let mut changed = false;
    for device in discovered {
        if devices.iter().any(|entry| entry.peer_id == device.peer_id) {
            continue;
        }
        tracing::info!(
            "Familiar peer {} reachable at {}:{} via TCP probe (mDNS miss)",
            device.name,
            device.ip,
            device.port
        );
        devices.push(device);
        changed = true;
    }

    if !changed {
        return;
    }

    let updated = devices.clone();
    drop(devices);
    let _ = app.emit("lan-devices-changed", &updated);
}

pub(crate) async fn refresh_lan_presence(
    config: &Arc<Mutex<AppConfig>>,
    lan_devices: &Arc<Mutex<Vec<LanDevice>>>,
    connected_peer: &Arc<Mutex<Option<ConnectedPeerPayload>>>,
    tcp_port: u16,
    app: &tauri::AppHandle,
) {
    discover_trusted_peers_by_tcp_probe(config, lan_devices, tcp_port, app).await;
    reconcile_lan_devices(config, lan_devices, connected_peer, tcp_port, app).await;
}

pub(crate) fn spawn_lan_presence_refresh(state: &AppState, app: &tauri::AppHandle) {
    let config = state.config.clone();
    let lan_devices = state.lan_devices.clone();
    let connected_peer = state.connected_peer.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(LAN_PRESENCE_REFRESH_DELAY).await;
        let tcp_port = config
            .lock()
            .await
            .tcp_port
            .unwrap_or(app_profile::DEFAULT_TCP_PORT);
        refresh_lan_presence(&config, &lan_devices, &connected_peer, tcp_port, &app).await;
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
    let probe_ports = app_profile::tcp_probe_port_candidates(tcp_port);

    let lan_target = {
        let devices = state.lan_devices.lock().await;
        devices
            .iter()
            .find(|device| device.peer_id == peer_id)
            .map(|device| (device.ip.clone(), device.port))
    };

    if let Some((ip, port)) = lan_target {
        return direct::probe_tcp_reachable_resilient(&ip, port, &probe_ports, LAN_PROBE_TIMEOUT)
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
        return direct::probe_tcp_reachable_on_any_port(&ip, &probe_ports, LAN_PROBE_TIMEOUT)
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
    _config: &Arc<Mutex<AppConfig>>,
    lan_devices: &Arc<Mutex<Vec<LanDevice>>>,
    connected_peer: &Arc<Mutex<Option<ConnectedPeerPayload>>>,
    tcp_port: u16,
    app: &tauri::AppHandle,
) {
    let snapshot = lan_devices.lock().await.clone();
    if snapshot.is_empty() {
        return;
    }

    let probe_ports = app_profile::tcp_probe_port_candidates(tcp_port);
    let connected_peer_id = connected_peer
        .lock()
        .await
        .as_ref()
        .map(|peer| peer.peer_id.clone());

    let probe_results = futures_util::future::join_all(snapshot.iter().map(|device| {
        let peer_id = device.peer_id.clone();
        let ip = device.ip.clone();
        let port = device.port;
        let skip_probe = connected_peer_id.as_deref() == Some(peer_id.as_str());
        let probe_ports = probe_ports.clone();
        async move {
            if skip_probe {
                return (peer_id, true, None);
            }
            match direct::probe_tcp_reachable_resilient(&ip, port, &probe_ports, LAN_PROBE_TIMEOUT)
                .await
            {
                Some(found_port) => (peer_id, true, Some(found_port)),
                None => (peer_id, false, None),
            }
        }
    }))
    .await;

    let unreachable: std::collections::HashSet<_> = probe_results
        .iter()
        .filter(|(_, reachable, _)| !reachable)
        .map(|(peer_id, _, _)| peer_id.clone())
        .collect();

    let port_updates: std::collections::HashMap<_, _> = probe_results
        .into_iter()
        .filter_map(|(peer_id, reachable, found_port)| {
            if reachable {
                found_port.map(|port| (peer_id, port))
            } else {
                None
            }
        })
        .collect();

    let mut devices = lan_devices.lock().await;
    let mut changed = false;

    for device in devices.iter_mut() {
        if let Some(found_port) = port_updates.get(&device.peer_id) {
            if device.port != *found_port {
                tracing::info!(
                    "LAN device {} reachable on updated port {} (was {})",
                    device.name,
                    found_port,
                    device.port
                );
                device.port = *found_port;
                changed = true;
            }
        }
    }

    if !unreachable.is_empty() {
        let before = devices.len();
        devices.retain(|device| !unreachable.contains(&device.peer_id));
        if devices.len() != before {
            tracing::info!(
                "Pruned {} unreachable LAN device(s) after TCP probe",
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
        auto_connect_trusted: config.auto_connect_trusted.unwrap_or(false),
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
    SyncSettingsPayload {
        sync_images: config.sync_images.unwrap_or(true),
        sync_files: config.sync_files.unwrap_or(true),
        max_file_mb: max_file_bytes_to_mb(
            config
                .max_file_bytes
                .unwrap_or(DEFAULT_MAX_FILE_BYTES),
        ),
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

fn clipboard_settings_from_config(config: &AppConfig) -> ClipboardSettingsPayload {
    ClipboardSettingsPayload {
        history_limit: normalized_clipboard_history_limit(config.clipboard_history_limit),
    }
}

fn load_clipboard_history_from_config(config: &AppConfig) -> Vec<ClipboardHistoryEntry> {
    let limit = normalized_clipboard_history_limit(config.clipboard_history_limit);
    config
        .clipboard_history
        .clone()
        .unwrap_or_default()
        .into_iter()
        .take(limit)
        .collect()
}

pub(crate) async fn store_connected_peer(
    connected_peer: &Arc<Mutex<Option<ConnectedPeerPayload>>>,
    peer_name: String,
    peer_id: String,
) {
    *connected_peer.lock().await = Some(ConnectedPeerPayload {
        peer_name,
        peer_id,
    });
}

async fn clear_connected_peer(connected_peer: &Arc<Mutex<Option<ConnectedPeerPayload>>>) {
    *connected_peer.lock().await = None;
}

fn next_connection_generation(state: &AppState) -> u64 {
    state.connection_generation.fetch_add(1, Ordering::SeqCst) + 1
}

async fn is_duplicate_active_session(
    connected: &Arc<Mutex<bool>>,
    connected_peer: &Arc<Mutex<Option<ConnectedPeerPayload>>>,
    peer_id: &str,
) -> bool {
    if !*connected.lock().await {
        return false;
    }

    connected_peer
        .lock()
        .await
        .as_ref()
        .is_some_and(|peer| peer.peer_id() == peer_id)
}

async fn persist_clipboard_history(
    config: &Arc<Mutex<AppConfig>>,
    history: &[ClipboardHistoryEntry],
) {
    let mut cfg = config.lock().await;
    cfg.clipboard_history = Some(history.to_vec());
    storage_json::save_config(&cfg);
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
            })
        }
        clipboard::types::ClipboardSnapshot::Image {
            png_bytes,
            width,
            height,
        } => Some(ClipboardHistoryEntry {
            id: format!("{}-{}", event.timestamp_ms, &hash[..8]),
            content: format!("[图片] {width}×{height}"),
            clip_type: "image".to_string(),
            source_label,
            direction,
            timestamp_ms: event.timestamp_ms,
            size_label: Some(clipboard::image::format_byte_size(png_bytes.len())),
            image_data_url: Some(clipboard::image::png_data_url(png_bytes)),
        }),
        clipboard::types::ClipboardSnapshot::FileList { files } => {
            let is_single_image =
                files.len() == 1 && clipboard::file::is_image_file_name(&files[0].file_name);
            Some(ClipboardHistoryEntry {
                id: format!("{}-{}", event.timestamp_ms, &hash[..8]),
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
                image_data_url: clipboard::file::history_preview_for_files(files),
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
    history.truncate(limit);
}

async fn active_connection_count(state: &AppState) -> usize {
    if *state.connected.lock().await {
        1
    } else {
        0
    }
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

    if is_duplicate_active_session(&state.connected, &state.connected_peer, &peer_id).await {
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
    let handle = ConnectionManager::connect_direct(
        conn,
        state.connected.clone(),
        state.connection.clone(),
        state.connected_peer.clone(),
        state.connection_generation.clone(),
        session_generation,
        state.clip_tx.clone(),
        app.clone(),
    )
    .await;
    *state.connection.lock().await = Some(handle);
    store_connected_peer(&state.connected_peer, peer_name.clone(), peer_id.clone()).await;

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
    let connected = *state.connected.lock().await;
    if !connected {
        clear_connected_peer(&state.connected_peer).await;
        return Ok(None);
    }

    Ok(state.connected_peer.lock().await.clone())
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
) -> Result<SyncSettingsPayload, String> {
    {
        let mut config = state.config.lock().await;
        config.sync_images = Some(sync_images);
        if let Some(sync_files) = sync_files {
            config.sync_files = Some(sync_files);
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
async fn get_status(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let connected = *state.connected.lock().await;
    if connected {
        Ok("connected".into())
    } else {
        Ok("disconnected".into())
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
    history_limit: usize,
) -> Result<ClipboardSettingsPayload, String> {
    let history_limit = validate_clipboard_history_limit(history_limit)?;

    {
        let mut config = state.config.lock().await;
        config.clipboard_history_limit = Some(history_limit);
        storage_json::save_config(&config);
    }

    let updated_history = {
        let mut history = state.clipboard_history.lock().await;
        history.truncate(history_limit);
        history.clone()
    };
    persist_clipboard_history(&state.config, &updated_history).await;
    let _ = app.emit("clipboard-history-changed", updated_history);

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
    let _ = app.emit("clipboard-history-changed", updated_history);
    Ok(())
}

#[tauri::command]
async fn get_ui_settings(state: tauri::State<'_, AppState>) -> Result<UiSettingsPayload, String> {
    let config = state.config.lock().await;
    Ok(ui_settings_from_config(&config))
}

#[tauri::command]
async fn get_clipboard_history(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ClipboardHistoryEntry>, String> {
    let history = state.clipboard_history.lock().await.clone();
    Ok(history)
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
        state.connected.clone(),
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

    *state.connection.lock().await = Some(handle);

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
        &state.lan_devices,
        &state.connected_peer,
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
            let handle = ConnectionManager::connect_direct(
                conn,
                state.connected.clone(),
                state.connection.clone(),
                state.connected_peer.clone(),
                state.connection_generation.clone(),
                session_generation,
                state.clip_tx.clone(),
                app.clone(),
            )
            .await;
            *state.connection.lock().await = Some(handle);
            store_connected_peer(
                &state.connected_peer,
                peer_name.clone(),
                peer_id.clone(),
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

    let stream = state
        .pending_initiator
        .lock()
        .await
        .take()
        .ok_or("当前没有待处理的连接")?;

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
    let disconnected_peer_name = state
        .connected_peer
        .lock()
        .await
        .as_ref()
        .map(|peer| peer.peer_name().to_string());

    next_connection_generation(state.inner());
    abort_outbound_handshake(state.inner()).await;

    let local_peer_id = {
        let key_pair = state.key_pair.lock().await;
        key_pair
            .as_ref()
            .map(|pair| peer_id_from_public_key(pair.public.as_bytes()))
            .unwrap_or_default()
    };

    if let Some(handle) = state.connection.lock().await.take() {
        handle.notify_peer_left(&local_peer_id);
        tokio::time::sleep(Duration::from_millis(120)).await;
    }

    *state.connected.lock().await = false;
    clear_connected_peer(&state.connected_peer).await;

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

    if let Some(peer_name) = disconnected_peer_name {
        let trimmed = peer_name.trim();
        let message = if trimmed.is_empty() {
            "设备 已断开连接".to_string()
        } else {
            format!("{trimmed} 已断开连接")
        };
        window::send_session_ended_notification(&app, &message);
    }

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

async fn finalize_incoming_connection(
    conn: network::direct::DirectConnection,
    initiator_name: String,
    initiator_peer_id: String,
    initiator_pk: Vec<u8>,
    initiator_ip: Option<String>,
    is_reconnect: bool,
    config: Arc<Mutex<AppConfig>>,
    connected: Arc<Mutex<bool>>,
    connected_peer: Arc<Mutex<Option<ConnectedPeerPayload>>>,
    connection: Arc<Mutex<Option<ConnectionHandle>>>,
    connection_generation: Arc<AtomicU64>,
    clip_tx: broadcast::Sender<ClipboardEvent>,
    app_handle: &tauri::AppHandle,
) {
    if is_duplicate_active_session(&connected, &connected_peer, &initiator_peer_id).await {
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
    let handle = ConnectionManager::connect_direct(
        conn,
        connected.clone(),
        connection.clone(),
        connected_peer.clone(),
        connection_generation,
        session_generation,
        clip_tx.clone(),
        app_handle.clone(),
    )
    .await;
    *connection.lock().await = Some(handle);
    store_connected_peer(
        &connected_peer,
        initiator_name.clone(),
        initiator_peer_id.clone(),
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
    connected: Arc<Mutex<bool>>,
    connected_peer: Arc<Mutex<Option<ConnectedPeerPayload>>>,
    connection: Arc<Mutex<Option<ConnectionHandle>>>,
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
                    connected,
                    connected_peer,
                    connection,
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
                    connected,
                    connected_peer,
                    connection,
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
                    connected,
                    connected_peer,
                    connection,
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

fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    // Dev builds hide mdns-sd interface noise on multi-NIC hosts; override via RUST_LOG.
    let default_filter = if cfg!(debug_assertions) {
        "info,mdns_sd=off"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(default_filter)),
        )
        .init();
}

pub fn run() {
    init_tracing();

    match storage::staging::gc_staging() {
        Ok(removed) if removed > 0 => tracing::info!("staging GC removed {removed} expired item(s)"),
        Err(error) => tracing::warn!("staging GC failed: {error}"),
        _ => {}
    }

    let mut config = storage_json::load_config();
    config.device_name = normalize_stored_device_name(&config.device_name);
    normalize_trusted_peers(&mut config);

    let key_pair = load_or_create_key_pair(&mut config);
    storage_json::save_config(&config);

    let tcp_port = config.tcp_port.unwrap_or(app_profile::DEFAULT_TCP_PORT);
    let initial_clipboard_history = load_clipboard_history_from_config(&config);

    let (clip_tx, _) = broadcast::channel::<ClipboardEvent>(16);

    let app_state = AppState {
        config: Arc::new(Mutex::new(config)),
        key_pair: Arc::new(Mutex::new(Some(key_pair.clone()))),
        connected: Arc::new(Mutex::new(false)),
        connected_peer: Arc::new(Mutex::new(None)),
        connection: Arc::new(Mutex::new(None)),
        connection_generation: Arc::new(AtomicU64::new(0)),
        clipboard_monitor_generation: Arc::new(AtomicU64::new(0)),
        clipboard_dedup_baseline: Arc::new(Mutex::new(None)),
        clip_tx: clip_tx.clone(),
        clipboard_history: Arc::new(Mutex::new(initial_clipboard_history)),
        lan_devices: Arc::new(Mutex::new(Vec::new())),
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

            let _ = app.handle().notification().request_permission();

            let startup = startup_settings_from_config(&app.state::<AppState>().config.blocking_lock());
            if let Err(error) = sync_autostart(app.handle(), startup.launch_at_startup) {
                tracing::warn!("Failed to sync autostart on startup: {}", error);
            }
            if !startup.silent_start {
                window::ensure_main_window(app.handle().clone());
            }

            if let Some(win) = app.get_webview_window(window::MAIN_WINDOW_LABEL) {
                window::attach_main_window_close_handler(app.handle().clone(), win);
            }

            if startup.silent_start {
                window::destroy_main_window(app.handle());
            }

            let toggle = MenuItemBuilder::with_id("toggle", format!("打开 {APP_DISPLAY_NAME}")).build(app)?;
            let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&toggle, &separator, &quit])
                .build()?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(format!("{APP_DISPLAY_NAME} 剪贴板同步"))
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
            let device_name = normalize_stored_device_name(&app.state::<AppState>().config.blocking_lock().device_name);
            let peer_id = key_pair.fingerprint();
            let lan_devices = app.state::<AppState>().lan_devices.clone();
            let clipboard_history = app.state::<AppState>().clipboard_history.clone();
            let pending_accept_tx = app.state::<AppState>().pending_accept_tx.clone();
            let pending_reject_tx = app.state::<AppState>().pending_reject_tx.clone();
            let connected = app.state::<AppState>().connected.clone();
            let connected_peer = app.state::<AppState>().connected_peer.clone();
            let connection = app.state::<AppState>().connection.clone();
            let connection_generation = app.state::<AppState>().connection_generation.clone();
            let config = app.state::<AppState>().config.clone();

            let (discovery_tx, mut discovery_rx) = mpsc::unbounded_channel::<DiscoveryEvent>();

            match discovery::start_discovery(&device_name, &peer_id, tcp_port, discovery_tx) {
                Ok(daemon) => {
                    std::thread::spawn(move || {
                        let _daemon = daemon;
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(3600));
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to start mDNS discovery: {}", e);
                }
            }

            {
                let lan_devices = lan_devices.clone();
                let app_handle = app_handle.clone();
                let auto_connect_deps = auto_connect::AutoConnectDeps {
                    config: config.clone(),
                    key_pair: app.state::<AppState>().key_pair.clone(),
                    connected: connected.clone(),
                    connected_peer: connected_peer.clone(),
                    connection: connection.clone(),
                    connection_generation: connection_generation.clone(),
                    clip_tx: clip_tx.clone(),
                    pending_initiator: app.state::<AppState>().pending_initiator.clone(),
                    pending_outbound: app.state::<AppState>().pending_outbound.clone(),
                    outbound_abort: app.state::<AppState>().outbound_abort.clone(),
                    outbound_handshake_active: app
                        .state::<AppState>()
                        .outbound_handshake_active
                        .clone(),
                    pending_connection_request: app.state::<AppState>().pending_connection_request.clone(),
                    tcp_port,
                };
                tauri::async_runtime::spawn(async move {
                    while let Some(event) = discovery_rx.recv().await {
                        let mut devices = lan_devices.lock().await;
                        let mut changed = false;
                        let mut added_or_refreshed: Option<LanDevice> = None;
                        match event {
                            DiscoveryEvent::Added(dev) => {
                                if let Some(existing) =
                                    devices.iter_mut().find(|d| d.peer_id == dev.peer_id)
                                {
                                    if *existing != dev {
                                        tracing::info!(
                                            "LAN device refreshed: {} ({})",
                                            dev.name,
                                            dev.ip
                                        );
                                        *existing = dev.clone();
                                        changed = true;
                                        added_or_refreshed = Some(dev);
                                    }
                                } else {
                                    tracing::info!("LAN device added: {} ({})", dev.name, dev.ip);
                                    devices.push(dev.clone());
                                    changed = true;
                                    added_or_refreshed = Some(dev);
                                }
                            }
                            DiscoveryEvent::Removed { service_fullname } => {
                                let before = devices.len();
                                devices.retain(|d| !lan_device_matches_removal(d, &service_fullname));
                                if devices.len() != before {
                                    tracing::info!("LAN device removed: {}", service_fullname);
                                    changed = true;
                                }
                            }
                        }
                        if changed {
                            let _ = app_handle.emit("lan-devices-changed", &*devices);
                        }
                        if let Some(device) = added_or_refreshed {
                            let deps = auto_connect::AutoConnectDeps {
                                config: auto_connect_deps.config.clone(),
                                key_pair: auto_connect_deps.key_pair.clone(),
                                connected: auto_connect_deps.connected.clone(),
                                connected_peer: auto_connect_deps.connected_peer.clone(),
                                connection: auto_connect_deps.connection.clone(),
                                connection_generation: auto_connect_deps.connection_generation.clone(),
                                clip_tx: auto_connect_deps.clip_tx.clone(),
                                pending_initiator: auto_connect_deps.pending_initiator.clone(),
                                pending_outbound: auto_connect_deps.pending_outbound.clone(),
                                outbound_abort: auto_connect_deps.outbound_abort.clone(),
                                outbound_handshake_active: auto_connect_deps
                                    .outbound_handshake_active
                                    .clone(),
                                pending_connection_request: auto_connect_deps
                                    .pending_connection_request
                                    .clone(),
                                tcp_port: auto_connect_deps.tcp_port,
                            };
                            auto_connect::maybe_auto_connect_discovered_device(
                                &deps,
                                &app_handle,
                                &device,
                            )
                            .await;
                        }
                    }
                });
            }

            {
                let probe_lan_devices = lan_devices.clone();
                let probe_connected_peer = connected_peer.clone();
                let probe_config = config.clone();
                let probe_app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    loop {
                        refresh_lan_presence(
                            &probe_config,
                            &probe_lan_devices,
                            &probe_connected_peer,
                            tcp_port,
                            &probe_app_handle,
                        )
                        .await;
                        tokio::time::sleep(LAN_PROBE_INTERVAL).await;
                    }
                });
            }

            {
                let startup_app_handle = app_handle.clone();
                let startup_deps = auto_connect::AutoConnectDeps {
                    config: config.clone(),
                    key_pair: app.state::<AppState>().key_pair.clone(),
                    connected: connected.clone(),
                    connected_peer: connected_peer.clone(),
                    connection: connection.clone(),
                    connection_generation: connection_generation.clone(),
                    clip_tx: clip_tx.clone(),
                    pending_initiator: app.state::<AppState>().pending_initiator.clone(),
                    pending_outbound: app.state::<AppState>().pending_outbound.clone(),
                    outbound_abort: app.state::<AppState>().outbound_abort.clone(),
                    outbound_handshake_active: app
                        .state::<AppState>()
                        .outbound_handshake_active
                        .clone(),
                    pending_connection_request: app.state::<AppState>().pending_connection_request.clone(),
                    tcp_port,
                };
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

            let (listener_tx, mut listener_rx) = mpsc::unbounded_channel::<ListenerEvent>();

            tauri::async_runtime::spawn(async move {
                if let Err(e) = direct::run_listener(tcp_port, listener_tx).await {
                    tracing::error!("TCP listener error: {}", e);
                }
            });

            {
                let app_handle = app_handle.clone();
                let connected = connected.clone();
                let connected_peer = connected_peer.clone();
                let connection = connection.clone();
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
                                    connected.clone(),
                                    connected_peer.clone(),
                                    connection.clone(),
                                    clip_tx.clone(),
                                    pending_accept_tx.clone(),
                                    pending_reject_tx.clone(),
                                )
                                .await;
                            }
                        }
                    }
                });
            }

            {
                let app_handle = app_handle.clone();
                let clipboard_history = clipboard_history.clone();
                let config = config.clone();
                let mut clip_history_rx = clip_tx.subscribe();

                tauri::async_runtime::spawn(async move {
                    while let Ok(event) = clip_history_rx.recv().await {
                        if let Some(entry) = build_clipboard_history_entry(&event) {
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
                        }
                    }
                });
            }

            let clip_tx_monitor = clip_tx.clone();
            let clip_rx = clip_tx.subscribe();
            let connection_bg = connection.clone();
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
                    tokio::join!(
                        async move {
                            let mut monitor = ClipboardMonitor::new(
                                clip_tx_monitor,
                                config_monitor,
                                clipboard_monitor_generation,
                                clipboard_dedup_baseline,
                            );
                            monitor.run().await;
                        },
                        async move {
                            let engine = sync::engine::SyncEngine::new(
                                clip_rx,
                                connection_bg,
                                config_bg,
                                app_handle_bg,
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
            get_pairing_code,
            rotate_pairing_code,
            end_pairing_session,
            get_ui_settings,
            get_startup_settings,
            save_startup_settings,
            get_connection_settings,
            save_connection_settings,
            get_sync_settings,
            save_sync_settings,
            get_clipboard_settings,
            save_clipboard_settings,
            get_clipboard_history,
            clear_clipboard_history,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
