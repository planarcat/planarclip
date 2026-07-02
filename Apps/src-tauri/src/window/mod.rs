use crate::app_profile::APP_DISPLAY_NAME;
use crate::AppState;
use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Emitter, Manager, UserAttentionType, WindowEvent};
#[cfg(not(windows))]
use tauri_plugin_notification::NotificationExt;

pub const MAIN_WINDOW_LABEL: &str = "main";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnectionRequestPayload {
    pub device_name: String,
    pub peer_id: String,
    #[serde(default)]
    pub requires_pairing: bool,
}

pub fn is_main_window_visible(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN_WINDOW_LABEL)
        .map(|window| window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false))
        .unwrap_or(false)
}

/// 用户主动打开（托盘/菜单）：显示窗口并获取焦点。
pub fn ensure_main_window(app: AppHandle) {
    present_main_window(app, true);
}

/// 入站连接请求：窗口出现在任务栏并闪烁提示，但不抢夺当前前台焦点。
pub fn ensure_main_window_in_background(app: AppHandle) {
    present_main_window(app, false);
}

fn present_main_window(app: AppHandle, steal_focus: bool) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        if steal_focus {
            let _ = window.set_focus();
        } else {
            flash_taskbar_attention(&window);
        }
        return;
    }

    if let Err(error) = build_main_window(&app, steal_focus) {
        tracing::error!("Failed to create main window: {}", error);
    }
}

/// Hides the main window but keeps the WebView alive so reopening from the tray is instant.
pub fn hide_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    let _ = window.hide();
}

pub fn toggle_main_window(app: &AppHandle) {
    if is_main_window_visible(app) {
        hide_main_window(app);
    } else {
        ensure_main_window(app.clone());
    }
}

pub fn attach_main_window_close_handler(app: AppHandle, window: tauri::WebviewWindow) {
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let exit_on_close = app
                .try_state::<AppState>()
                .map(|state| {
                    let config = state.config.blocking_lock();
                    normalized_close_window_action(config.close_window_action.as_deref()) == "exit"
                })
                .unwrap_or(false);

            if exit_on_close {
                app.exit(0);
                return;
            }

            hide_main_window(&app);
        }
    });
}

fn normalized_close_window_action(value: Option<&str>) -> &'static str {
    match value {
        Some("exit") => "exit",
        Some("tray") => "tray",
        _ => "tray",
    }
}

fn system_notifications_enabled(app: &AppHandle) -> bool {
    app.try_state::<AppState>()
        .map(|state| {
            state
                .config
                .blocking_lock()
                .system_notifications_enabled
                .unwrap_or(true)
        })
        .unwrap_or(true)
}

pub fn send_connection_notification(app: &AppHandle, device_name: &str) {
    let body = format!("{device_name} 已请求连接，请在任务栏中确认");
    show_planarclip_notification(app, &body, false);
}

pub fn send_session_established_notification(
    app: &AppHandle,
    device_name: &str,
    _is_reconnect: bool,
) {
    let trimmed = device_name.trim();
    let body = if trimmed.is_empty() {
        "设备 已连接".to_string()
    } else {
        format!("{trimmed} 已连接")
    };
    show_planarclip_notification(app, &body, false);
}

pub fn send_session_ended_notification(app: &AppHandle, message: &str) {
    let body = message.trim();
    if body.is_empty() {
        send_user_notification(app, "设备 已断开连接", true);
    } else {
        send_user_notification(app, body, true);
    }
}

pub fn send_user_notification(app: &AppHandle, body: &str, important: bool) {
    show_planarclip_notification(app, body, important);
}

fn show_planarclip_notification(app: &AppHandle, body: &str, important: bool) {
    if !system_notifications_enabled(app) {
        return;
    }

    #[cfg(windows)]
    {
        let title = app
            .config()
            .product_name
            .clone()
            .unwrap_or_else(|| APP_DISPLAY_NAME.to_string());
        let app_id = app.config().identifier.clone();
        crate::platform::windows::show_toast(&app_id, &title, body, important);
        return;
    }

    #[cfg(not(windows))]
    if let Err(error) = app
        .notification()
        .builder()
        .title(APP_DISPLAY_NAME)
        .body(body)
        .show()
    {
        tracing::warn!("Failed to show system notification: {}", error);
    }
}

pub async fn present_connection_request(
    app: &AppHandle,
    pending: &std::sync::Arc<tokio::sync::Mutex<Option<ConnectionRequestPayload>>>,
    device_name: String,
    peer_id: String,
    requires_pairing: bool,
) {
    let request = ConnectionRequestPayload {
        device_name: device_name.clone(),
        peer_id,
        requires_pairing,
    };
    *pending.lock().await = Some(request.clone());

    let had_live_window = app.get_webview_window(MAIN_WINDOW_LABEL).is_some();
    if !is_main_window_visible(app) {
        send_connection_notification(app, &device_name);
    }

    ensure_main_window_in_background(app.clone());

    if had_live_window {
        let _ = app.emit("connection-request", &request);
    }
}

fn flash_taskbar_attention(window: &tauri::WebviewWindow) {
    if let Err(error) = window.request_user_attention(Some(UserAttentionType::Critical)) {
        tracing::warn!("Failed to request taskbar attention: {}", error);
    }
}

/// Creates or reveals the main window synchronously (used at app startup when not in silent-tray mode).
pub fn bootstrap_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    build_main_window(app, true)
}

fn build_main_window(app: &AppHandle, steal_focus: bool) -> Result<(), String> {
    if app.get_webview_window(MAIN_WINDOW_LABEL).is_some() {
        return Ok(());
    }

    let config = app
        .config()
        .app
        .windows
        .first()
        .ok_or_else(|| "missing main window config".to_string())?
        .clone();

    let mut builder = WebviewWindowBuilder::from_config(app, &config).map_err(|error| error.to_string())?;
    if !steal_focus {
        builder = builder.focused(false);
    }

    let window = builder.build().map_err(|error| error.to_string())?;

    attach_main_window_close_handler(app.clone(), window.clone());
    window.show().map_err(|error| error.to_string())?;

    if steal_focus {
        window.set_focus().map_err(|error| error.to_string())?;
    } else {
        flash_taskbar_attention(&window);
    }

    Ok(())
}
