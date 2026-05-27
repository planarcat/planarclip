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
use storage::json::{AppConfig, load_config, save_config};
use sync::engine::SyncEngine;

pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub key_pair: Arc<Mutex<Option<KeyPair>>>,
    pub connected: Arc<Mutex<bool>>,
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
        let code = kp.fingerprint();
        // Use last 6 chars of fingerprint as pairing code
        let numeric: String = code.chars().filter(|c| c.is_ascii_digit()).collect();
        let pairing = if numeric.len() >= 6 {
            numeric[numeric.len()-6..].to_string()
        } else {
            format!("{:06}", u32::from_str_radix(&code[..6], 16).unwrap_or(0) % 1_000_000)
        };
        Ok(pairing)
    } else {
        Err("Key pair not initialized".into())
    }
}

#[tauri::command]
async fn pair(state: tauri::State<'_, AppState>, code: String) -> Result<String, String> {
    if code.len() != 6 {
        return Err("Pairing code must be 6 digits".into());
    }
    // In MVP: accept any 6-digit code, mark as connected
    // Full implementation would validate the code cryptographically
    let mut connected = state.connected.lock().await;
    *connected = true;
    Ok("paired".into())
}

pub fn run() {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Load or create app config
    let mut config = load_config();

    // Generate key pair if not exists
    let key_pair = KeyPair::generate();
    if config.device_name.is_empty() {
        config.device_name = "My Device".into();
    }
    save_config(&config);

    let app_state = AppState {
        config: Arc::new(Mutex::new(config)),
        key_pair: Arc::new(Mutex::new(Some(key_pair))),
        connected: Arc::new(Mutex::new(false)),
    };

    // Clipboard broadcast channel
    let (clip_tx, _) = broadcast::channel::<ClipboardSnapshot>(16);

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
            let connected = app.state::<AppState>().connected.clone();
            tokio::spawn(async move {
                let engine = SyncEngine::new(clip_rx, connected);
                engine.run().await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_status, get_pairing_code, pair])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
