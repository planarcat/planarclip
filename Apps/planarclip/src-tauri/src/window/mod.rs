use crate::app_profile::APP_DISPLAY_NAME;
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

    tauri::async_runtime::spawn(async move {
        if let Err(error) = recreate_main_window(&app, steal_focus).await {
            tracing::error!("Failed to recreate main window: {}", error);
        }
    });
}

pub fn destroy_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        if let Err(error) = window.destroy() {
            tracing::warn!("Failed to destroy main window: {}", error);
        }
    });
}

pub fn toggle_main_window(app: &AppHandle) {
    if is_main_window_visible(app) {
        destroy_main_window(app);
    } else {
        ensure_main_window(app.clone());
    }
}

pub fn attach_main_window_close_handler(app: AppHandle, window: tauri::WebviewWindow) {
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            // 关窗时销毁 WebView，释放 Chromium 内存；同步逻辑继续在 Rust 后端运行。
            api.prevent_close();
            destroy_main_window(&app);
        }
    });
}

pub fn send_connection_notification(app: &AppHandle, device_name: &str) {
    let body = format!("{device_name} 请求连接，请点击任务栏中的 {APP_DISPLAY_NAME} 确认");
    show_planarclip_notification(app, &body, false);
}

pub fn send_session_established_notification(
    app: &AppHandle,
    device_name: &str,
    is_reconnect: bool,
) {
    let trimmed = device_name.trim();
    let body = if trimmed.is_empty() {
        if is_reconnect {
            "已恢复与熟悉设备的连接。".to_string()
        } else {
            "已建立连接，剪贴板同步已开启。".to_string()
        }
    } else if is_reconnect {
        format!("已恢复与 {trimmed} 的连接。")
    } else {
        format!("已与 {trimmed} 建立连接，剪贴板同步已开启。")
    };
    show_planarclip_notification(app, &body, false);
}

pub fn send_session_ended_notification(app: &AppHandle, message: &str) {
    let body = message.trim();
    if body.is_empty() {
        show_planarclip_notification(app, "与设备的连接已断开。", true);
    } else {
        show_planarclip_notification(app, body, true);
    }
}

fn show_planarclip_notification(app: &AppHandle, body: &str, important: bool) {
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

async fn recreate_main_window(app: &AppHandle, steal_focus: bool) -> Result<(), String> {
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
