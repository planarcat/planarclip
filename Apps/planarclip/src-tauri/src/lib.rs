use std::sync::Arc;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};

mod clipboard;
mod crypto;
mod network;
mod storage;
mod sync;
mod tray;

use clipboard::monitor::ClipboardMonitor;
use clipboard::types::ClipboardSnapshot;
use crypto::keys::KeyPair;
use network::direct::{self, InitiatorResult, ListenerEvent};
use network::discovery::{self, DiscoveryEvent, LanDevice};
use network::webrtc::{ConnectionHandle, ConnectionManager};
use storage::json::{self as storage_json, AppConfig, KeyPairData, PeerData, TrustedPeerData};

const SIGNALLING_SERVER: &str = "ws://localhost:8765";
const DEFAULT_TCP_PORT: u16 = 19876;

pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub key_pair: Arc<Mutex<Option<KeyPair>>>,
    pub connected: Arc<Mutex<bool>>,
    pub connection: Arc<Mutex<Option<ConnectionHandle>>>,
    pub clip_tx: broadcast::Sender<ClipboardSnapshot>,
    pub lan_devices: Arc<Mutex<Vec<LanDevice>>>,
    pub pending_initiator: Arc<Mutex<Option<TcpStream>>>,
    pub pending_reject_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
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
    let kp_guard = state.key_pair.lock().await;
    if let Some(ref kp) = *kp_guard {
        let fp = kp.fingerprint();
        let numeric: String = fp.chars().filter(|c| c.is_ascii_digit()).collect();
        let pairing = if numeric.len() >= 6 {
            numeric[numeric.len() - 6..].to_string()
        } else {
            format!("{:06}", u32::from_str_radix(&fp[..6], 16).unwrap_or(0) % 1_000_000)
        };
        Ok(pairing)
    } else {
        Err("密钥对尚未初始化".into())
    }
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
async fn connect_lan(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    ip: String,
    port: u16,
) -> Result<String, String> {
    let (device_name, key_pair) = {
        let config = state.config.lock().await;
        let kp = state.key_pair.lock().await;
        let kp = kp.clone().ok_or("密钥对尚未初始化")?;
        (config.device_name.clone(), kp)
    };

    match direct::initiator_connect(&ip, port, &device_name, &key_pair).await {
        Ok(InitiatorResult::Connected(conn)) => {
            let peer_name = conn.peer_name.clone();
            let peer_id = conn.peer_id.clone();
            let peer_pk = conn.peer_public_key.clone();

            {
                let mut config = state.config.lock().await;
                let mut peers = config.trusted_peers.clone().unwrap_or_default();
                if !peers.iter().any(|p| p.public_key == peer_pk) {
                    peers.push(TrustedPeerData {
                        name: peer_name.clone(),
                        public_key: peer_pk,
                        peer_id: peer_id.clone(),
                        last_ip: Some(ip),
                    });
                    config.trusted_peers = Some(peers);
                    storage_json::save_config(&config);
                }
            }

            let handle = ConnectionManager::connect_direct(
                conn,
                state.connected.clone(),
                state.clip_tx.clone(),
            )
            .await;
            *state.connection.lock().await = Some(handle);

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
        Err(e) => Err(format!("连接失败：{}", e)),
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
                let mut peers = config.trusted_peers.clone().unwrap_or_default();
                if !peers.iter().any(|p| p.public_key == peer_pk) {
                    peers.push(TrustedPeerData {
                        name: peer_name.clone(),
                        public_key: peer_pk,
                        peer_id: peer_id.clone(),
                        last_ip: None,
                    });
                    config.trusted_peers = Some(peers);
                    storage_json::save_config(&config);
                }
            }

            let handle = ConnectionManager::connect_direct(
                conn,
                state.connected.clone(),
                state.clip_tx.clone(),
            )
            .await;
            *state.connection.lock().await = Some(handle);

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
            let _ = app.emit("connection-failed", format!("{}", e));
            Err(format!("配对失败：{}", e))
        }
    }
}

#[tauri::command]
async fn reject_connection(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.pending_reject_tx.lock().await.take();
    if let Some(tx) = tx {
        let _ = tx.send(());
    }
    Ok(())
}

#[tauri::command]
async fn disconnect(state: tauri::State<'_, AppState>) -> Result<(), String> {
    *state.connected.lock().await = false;
    *state.connection.lock().await = None;
    *state.pending_initiator.lock().await = None;

    let tx = state.pending_reject_tx.lock().await.take();
    if let Some(tx) = tx {
        let _ = tx.send(());
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

async fn handle_incoming_connection(
    req: direct::IncomingRequest,
    device_name: String,
    key_pair: KeyPair,
    trusted_peers: Vec<TrustedPeerData>,
    config: Arc<Mutex<AppConfig>>,
    app_handle: tauri::AppHandle,
    connected: Arc<Mutex<bool>>,
    connection: Arc<Mutex<Option<ConnectionHandle>>>,
    clip_tx: broadcast::Sender<ClipboardSnapshot>,
    pending_reject_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
) {
    let initiator_name = req.initiator_name.clone();
    let initiator_pk = req.initiator_public_key.clone();

    let is_trusted = trusted_peers.iter().any(|tp| tp.public_key == initiator_pk);

    if is_trusted {
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
                let handle = ConnectionManager::connect_direct(
                    conn,
                    connected.clone(),
                    clip_tx.clone(),
                )
                .await;
                *connection.lock().await = Some(handle);

                let _ = app_handle.emit(
                    "connection-established",
                    serde_json::json!({
                        "peer_name": initiator_name,
                        "peer_id": direct::short_fingerprint(&initiator_pk),
                        "is_reconnect": true,
                    }),
                );
            }
            Err(e) => {
                tracing::warn!("Auto-accept handshake failed: {}", e);
            }
        }
        return;
    }

    let pairing_code = direct::generate_pairing_code();
    tracing::info!(
        "Unknown peer {} — pairing code: {}",
        initiator_name,
        pairing_code
    );

    let _ = app_handle.emit(
        "connection-request",
        serde_json::json!({
            "device_name": initiator_name,
            "peer_id": req.initiator_peer_id,
            "pairing_code": pairing_code,
        }),
    );

    let (reject_tx, reject_rx) = oneshot::channel();
    *pending_reject_tx.lock().await = Some(reject_tx);

    match direct::responder_verify_code(
        req.stream,
        &device_name,
        &key_pair,
        initiator_name.clone(),
        initiator_pk.clone(),
        &pairing_code,
        reject_rx,
    )
    .await
    {
        Ok(conn) => {
            {
                let mut cfg = config.lock().await;
                let mut peers = cfg.trusted_peers.clone().unwrap_or_default();
                if !peers.iter().any(|p| p.public_key == initiator_pk) {
                    peers.push(TrustedPeerData {
                        name: initiator_name.clone(),
                        public_key: initiator_pk.clone(),
                        peer_id: direct::short_fingerprint(&initiator_pk),
                        last_ip: None,
                    });
                    cfg.trusted_peers = Some(peers);
                    storage_json::save_config(&cfg);
                }
            }

            let handle = ConnectionManager::connect_direct(
                conn,
                connected.clone(),
                clip_tx.clone(),
            )
            .await;
            *connection.lock().await = Some(handle);

            let _ = app_handle.emit(
                "connection-established",
                serde_json::json!({
                    "peer_name": initiator_name,
                    "peer_id": direct::short_fingerprint(&initiator_pk),
                    "is_reconnect": false,
                }),
            );
        }
        Err(e) => {
            tracing::info!("Pairing failed: {}", e);
            let _ = app_handle.emit("connection-failed", format!("{}", e));
        }
    }

    *pending_reject_tx.lock().await = None;
}

pub fn run() {
    tracing_subscriber::fmt::init();

    let mut config = storage_json::load_config();
    if config.device_name.is_empty() {
        config.device_name = "我的设备".into();
    }

    let key_pair = load_or_create_key_pair(&mut config);
    storage_json::save_config(&config);

    let tcp_port = config.tcp_port.unwrap_or(DEFAULT_TCP_PORT);
    let trusted_peers = config.trusted_peers.clone().unwrap_or_default();

    let (clip_tx, _) = broadcast::channel::<ClipboardSnapshot>(16);

    let app_state = AppState {
        config: Arc::new(Mutex::new(config)),
        key_pair: Arc::new(Mutex::new(Some(key_pair.clone()))),
        connected: Arc::new(Mutex::new(false)),
        connection: Arc::new(Mutex::new(None)),
        clip_tx: clip_tx.clone(),
        lan_devices: Arc::new(Mutex::new(Vec::new())),
        pending_initiator: Arc::new(Mutex::new(None)),
        pending_reject_tx: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
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
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            let device_name = app.state::<AppState>().config.blocking_lock().device_name.clone();
            let peer_id = key_pair.fingerprint();
            let lan_devices = app.state::<AppState>().lan_devices.clone();
            let pending_reject_tx = app.state::<AppState>().pending_reject_tx.clone();
            let connected = app.state::<AppState>().connected.clone();
            let connection = app.state::<AppState>().connection.clone();
            let config = app.state::<AppState>().config.clone();

            let (discovery_tx, mut discovery_rx) = mpsc::unbounded_channel::<DiscoveryEvent>();

            match discovery::start_discovery(&device_name, &peer_id, tcp_port, discovery_tx) {
                Ok(_daemon) => {
                    std::thread::spawn(move || loop {
                        std::thread::sleep(std::time::Duration::from_secs(3600));
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to start mDNS discovery: {}", e);
                }
            }

            {
                let lan_devices = lan_devices.clone();
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    while let Some(event) = discovery_rx.recv().await {
                        let mut devices = lan_devices.lock().await;
                        match event {
                            DiscoveryEvent::Added(dev) => {
                                if !devices.iter().any(|d| d.peer_id == dev.peer_id) {
                                    tracing::info!("LAN device added: {} ({})", dev.name, dev.ip);
                                    devices.push(dev);
                                }
                            }
                            DiscoveryEvent::Removed(dev) => {
                                devices.retain(|d| d.name != dev.name);
                                tracing::info!("LAN device removed: {}", dev.name);
                            }
                        }
                        let _ = app_handle.emit("lan-devices-changed", &*devices);
                    }
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
                let connection = connection.clone();
                let clip_tx = clip_tx.clone();
                let pending_reject_tx = pending_reject_tx.clone();
                let trusted_peers = trusted_peers.clone();
                let key_pair = key_pair.clone();
                let config = config.clone();

                tauri::async_runtime::spawn(async move {
                    while let Some(event) = listener_rx.recv().await {
                        match event {
                            ListenerEvent::Incoming(req) => {
                                handle_incoming_connection(
                                    req,
                                    device_name.clone(),
                                    key_pair.clone(),
                                    trusted_peers.clone(),
                                    config.clone(),
                                    app_handle.clone(),
                                    connected.clone(),
                                    connection.clone(),
                                    clip_tx.clone(),
                                    pending_reject_tx.clone(),
                                )
                                .await;
                            }
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
            get_pairing_code,
            pair,
            get_lan_devices,
            connect_lan,
            submit_pairing_code,
            reject_connection,
            disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
