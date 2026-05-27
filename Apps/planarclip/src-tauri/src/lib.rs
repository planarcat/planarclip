use std::sync::Arc;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::Manager;
use tokio::sync::{broadcast, Mutex};

mod clipboard;
mod crypto;
mod network;
mod storage;
mod sync;
mod tray;
mod util;

use clipboard::monitor::ClipboardMonitor;
use clipboard::types::ClipboardSnapshot;
use crypto::keys::KeyPair;
use storage::json::{self as storage_json, AppConfig, KeyPairData, PeerData};
use sync::engine::SyncEngine;
use network::webrtc::{ConnectionHandle, ConnectionManager};

/// Default signalling server for MVP development.
const SIGNALLING_SERVER: &str = "ws://localhost:8765";

pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub key_pair: Arc<Mutex<Option<KeyPair>>>,
    pub connected: Arc<Mutex<bool>>,
    pub connection: Arc<Mutex<Option<ConnectionHandle>>>,
    pub clip_tx: broadcast::Sender<ClipboardSnapshot>,
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
        Err("Key pair not initialized".into())
    }
}

#[tauri::command]
async fn pair(state: tauri::State<'_, AppState>, code: String) -> Result<String, String> {
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("Pairing code must be 6 digits".into());
    }

    let peer_id = {
        let kp = state.key_pair.lock().await;
        match *kp {
            Some(ref kp) => kp.fingerprint(),
            None => return Err("Key pair not initialized".into()),
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
    .map_err(|e| format!("Connection failed: {}", e))?;

    // Persist peer info
    {
        let mut config = state.config.lock().await;
        config.paired_peer = Some(PeerData {
            name: "Paired Device".into(),
            public_key: vec![],
        });
        storage_json::save_config(&config);
    }

    *state.connection.lock().await = Some(handle);

    Ok("paired".into())
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

    // Generate new key pair and persist
    let kp = KeyPair::generate();
    config.key_pair = Some(KeyPairData {
        secret_bytes: kp.secret.as_bytes().to_vec(),
        public_bytes: kp.public_bytes().to_vec(),
    });
    tracing::info!("Generated new key pair");
    kp
}

pub fn run() {
    tracing_subscriber::fmt::init();

    let mut config = storage_json::load_config();
    if config.device_name.is_empty() {
        config.device_name = "My Device".into();
    }

    let key_pair = load_or_create_key_pair(&mut config);
    storage_json::save_config(&config);

    let (clip_tx, _) = broadcast::channel::<ClipboardSnapshot>(16);

    let app_state = AppState {
        config: Arc::new(Mutex::new(config)),
        key_pair: Arc::new(Mutex::new(Some(key_pair))),
        connected: Arc::new(Mutex::new(false)),
        connection: Arc::new(Mutex::new(None)),
        clip_tx: clip_tx.clone(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
            // Build tray menu
            let toggle = MenuItemBuilder::with_id("toggle", "Show PlanarClip")
                .build(app)?;
            let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit")
                .build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&toggle, &separator, &quit])
                .build()?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("PlanarClip")
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
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
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
                    }
                })
                .build(app)?;

            // Spawn clipboard monitor
            let clip_tx_monitor = clip_tx.clone();
            tokio::spawn(async move {
                let mut monitor = ClipboardMonitor::new(clip_tx_monitor);
                monitor.run().await;
            });

            // Spawn sync engine
            let clip_rx = clip_tx.subscribe();
            let connection = app.state::<AppState>().connection.clone();
            tokio::spawn(async move {
                let engine = SyncEngine::new(clip_rx, connection);
                engine.run().await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_status, get_pairing_code, pair])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
