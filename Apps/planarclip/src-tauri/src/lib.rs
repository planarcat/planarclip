use std::sync::Arc;
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
mod storage;
mod sync;
mod tray;
mod window;

use clipboard::monitor::ClipboardMonitor;
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
const MAX_CLIPBOARD_HISTORY: usize = 12;
const MAX_CONNECTIONS: usize = 5;

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

pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub key_pair: Arc<Mutex<Option<KeyPair>>>,
    pub connected: Arc<Mutex<bool>>,
    pub connected_peer: Arc<Mutex<Option<ConnectedPeerPayload>>>,
    pub connection: Arc<Mutex<Option<ConnectionHandle>>>,
    pub clip_tx: broadcast::Sender<ClipboardEvent>,
    pub clipboard_history: Arc<Mutex<Vec<ClipboardHistoryEntry>>>,
    pub lan_devices: Arc<Mutex<Vec<LanDevice>>>,
    pub pending_initiator: Arc<Mutex<Option<TcpStream>>>,
    pub pending_outbound: Arc<Mutex<Option<TcpStream>>>,
    pub outbound_abort: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub pending_accept_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub pending_reject_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    pub pending_connection_request: Arc<Mutex<Option<window::ConnectionRequestPayload>>>,
    pub pairing_session_code: Arc<Mutex<Option<Arc<Mutex<String>>>>>,
    pub pairing_code_expires_at: Arc<Mutex<Option<i64>>>,
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
pub struct ConnectedPeerPayload {
    peer_name: String,
    peer_id: String,
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

fn load_clipboard_history_from_config(config: &AppConfig) -> Vec<ClipboardHistoryEntry> {
    config
        .clipboard_history
        .clone()
        .unwrap_or_default()
        .into_iter()
        .take(MAX_CLIPBOARD_HISTORY)
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

async fn clear_pairing_session(state: &AppState) {
    *state.pairing_session_code.lock().await = None;
    *state.pairing_code_expires_at.lock().await = None;
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
    let fp = kp.fingerprint();
    let numeric: String = fp.chars().filter(|c| c.is_ascii_digit()).collect();
    if numeric.len() >= 6 {
        numeric[numeric.len() - 6..].to_string()
    } else {
        format!(
            "{:06}",
            u32::from_str_radix(&fp[..6.min(fp.len())], 16).unwrap_or(0) % 1_000_000
        )
    }
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
    let content = event.snapshot.text()?.trim().to_string();
    if content.is_empty() {
        return None;
    }

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

    Some(ClipboardHistoryEntry {
        id: format!("{}-{}", event.timestamp_ms, &hash[..8]),
        content,
        source_label,
        direction,
        timestamp_ms: event.timestamp_ms,
    })
}

fn merge_clipboard_history(history: &mut Vec<ClipboardHistoryEntry>, entry: ClipboardHistoryEntry) {
    if history.first().map(|item| item.content.as_str()) == Some(entry.content.as_str())
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
    history.truncate(MAX_CLIPBOARD_HISTORY);
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

pub(crate) fn emit_connection_failed(app: &tauri::AppHandle, error: &direct::HandshakeError) {
    let _ = app.emit(
        "connection-failed",
        serde_json::json!({
            "kind": error.reason_code(),
            "message": error.user_message(),
        }),
    );
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
    emit_pairing_code_rotated(&app, &new_code, expires_at_ms);
    Ok(new_code)
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

            let handle = ConnectionManager::connect_direct(
                conn,
                state.connected.clone(),
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

            Ok("connected".into())
        }
        Ok(InitiatorResult::AwaitingCode { stream }) => {
            *state.pending_initiator.lock().await = Some(stream);
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
                        last_ip: None,
                        auto_accept: None,
                    },
                );
                storage_json::save_config(&config);
            }

            let handle = ConnectionManager::connect_direct(
                conn,
                state.connected.clone(),
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
                    "is_reconnect": false,
                }),
            );

            Ok("connected".into())
        }
        Err(e) => {
            emit_connection_failed(&app, &e);
            Err(e.user_message())
        }
    }
}

#[tauri::command]
async fn accept_connection(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.pending_accept_tx.lock().await.take();
    if let Some(tx) = tx {
        let _ = tx.send(());
        clear_pending_connection_request(state.inner()).await;
        Ok(())
    } else {
        Err("当前没有待确认的连接请求".into())
    }
}

#[tauri::command]
async fn reject_connection(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let _ = state.pending_accept_tx.lock().await.take();
    if let Some(tx) = state.pending_reject_tx.lock().await.take() {
        let _ = tx.send(()).await;
    }
    clear_pending_connection_request(state.inner()).await;
    Ok(())
}

#[tauri::command]
async fn disconnect(state: tauri::State<'_, AppState>) -> Result<(), String> {
    abort_outbound_handshake(state.inner()).await;

    *state.connected.lock().await = false;
    *state.connection.lock().await = None;
    clear_connected_peer(&state.connected_peer).await;

    if let Some(stream) = state.pending_initiator.lock().await.take() {
        direct::initiator_abort(stream).await;
    }

    let _ = state.pending_accept_tx.lock().await.take();
    if let Some(tx) = state.pending_reject_tx.lock().await.take() {
        let _ = tx.send(()).await;
    }

    clear_pairing_session(&state).await;

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
    clip_tx: broadcast::Sender<ClipboardEvent>,
    app_handle: &tauri::AppHandle,
) {
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

    let handle = ConnectionManager::connect_direct(
        conn,
        connected.clone(),
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
        *pending_accept_tx.lock().await = Some(accept_tx);
        *pending_reject_tx.lock().await = Some(reject_tx);

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
                    clip_tx,
                    &app_handle,
                )
                .await;
            }
            Err(e) => {
                tracing::info!("Incoming confirmation failed: {}", e);
                clear_pending_connection_request(app_handle.state::<AppState>().inner()).await;
                if !matches!(e, direct::HandshakeError::Cancelled) {
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
        *pending_accept_tx.lock().await = Some(accept_tx);
        *pending_reject_tx.lock().await = Some(reject_tx);

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
                    clip_tx,
                    &app_handle,
                )
                .await;
            }
            Err(e) => {
                tracing::info!("Incoming pairing failed: {}", e);
                clear_pending_connection_request(app_handle.state::<AppState>().inner()).await;
                if !matches!(e, direct::HandshakeError::Cancelled | direct::HandshakeError::InvalidCode) {
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
}

pub fn run() {
    tracing_subscriber::fmt::init();

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
        clip_tx: clip_tx.clone(),
        clipboard_history: Arc::new(Mutex::new(initial_clipboard_history)),
        lan_devices: Arc::new(Mutex::new(Vec::new())),
        pending_initiator: Arc::new(Mutex::new(None)),
        pending_outbound: Arc::new(Mutex::new(None)),
        outbound_abort: Arc::new(Mutex::new(None)),
        pending_accept_tx: Arc::new(Mutex::new(None)),
        pending_reject_tx: Arc::new(Mutex::new(None)),
        pending_connection_request: Arc::new(Mutex::new(None)),
        pairing_session_code: Arc::new(Mutex::new(None)),
        pairing_code_expires_at: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("PlanarClip")
                .build(),
        )
        .manage(app_state)
        .setup(move |app| {
            use tauri_plugin_notification::NotificationExt;

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

            let toggle = MenuItemBuilder::with_id("toggle", "打开 PlanarClip").build(app)?;
            let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&toggle, &separator, &quit])
                .build()?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("PlanarClip 剪贴板同步")
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
                    clip_tx: clip_tx.clone(),
                    pending_initiator: app.state::<AppState>().pending_initiator.clone(),
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
                                clip_tx: auto_connect_deps.clip_tx.clone(),
                                pending_initiator: auto_connect_deps.pending_initiator.clone(),
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
                let startup_app_handle = app_handle.clone();
                let startup_deps = auto_connect::AutoConnectDeps {
                    config: config.clone(),
                    key_pair: app.state::<AppState>().key_pair.clone(),
                    connected: connected.clone(),
                    connected_peer: connected_peer.clone(),
                    connection: connection.clone(),
                    clip_tx: clip_tx.clone(),
                    pending_initiator: app.state::<AppState>().pending_initiator.clone(),
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
                                let mut history = clipboard_history.lock().await;
                                merge_clipboard_history(&mut history, entry);
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
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create background runtime");
                rt.block_on(async move {
                    tokio::join!(
                        async {
                            let mut monitor = ClipboardMonitor::new(clip_tx_monitor);
                            monitor.run().await;
                        },
                        async {
                            let engine = sync::engine::SyncEngine::new(clip_rx, connection_bg);
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
            get_ui_settings,
            get_startup_settings,
            save_startup_settings,
            get_connection_settings,
            save_connection_settings,
            get_clipboard_history,
            get_pending_connection_request,
            save_ui_settings,
            pair,
            get_lan_devices,
            get_trusted_peers,
            remove_trusted_peer,
            set_peer_auto_accept,
            connect_lan,
            submit_pairing_code,
            accept_connection,
            reject_connection,
            disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
