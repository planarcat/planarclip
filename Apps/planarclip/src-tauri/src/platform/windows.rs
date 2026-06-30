use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;
use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

pub mod clipboard;
pub mod thumbnail;

/// Registers this process with Windows so Toast notifications use the app identity
/// and the app appears under Settings → System → Notifications.
pub fn register_app_user_model_id(app_id: &str, display_name: &str) {
    let app_id = app_id.trim();
    if app_id.is_empty() {
        tracing::warn!("Skipped Windows notification registration: empty app id");
        return;
    }

    register_app_user_model_id_registry(app_id, display_name);
    pin_process_app_user_model_id(app_id);
}

fn register_app_user_model_id_registry(app_id: &str, display_name: &str) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path = format!(r"Software\Classes\AppUserModelId\{app_id}");

    match hkcu.create_subkey(&key_path) {
        Ok((key, _)) => {
            if let Err(error) = key.set_value("DisplayName", &display_name) {
                tracing::warn!(
                    "Failed to write notification DisplayName for {}: {}",
                    app_id,
                    error
                );
            } else {
                tracing::info!(
                    "Registered Windows notification identity {} ({})",
                    app_id,
                    display_name
                );
            }
        }
        Err(error) => {
            tracing::warn!(
                "Failed to create AppUserModelId registry key for {}: {}",
                app_id,
                error
            );
        }
    }
}

fn pin_process_app_user_model_id(app_id: &str) {
    let wide = wide_null(app_id);
    // SAFETY: SetCurrentProcessExplicitAppUserModelID accepts a null-terminated UTF-16 string.
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(wide.as_ptr()) };
    if result != 0 {
        tracing::warn!(
            "SetCurrentProcessExplicitAppUserModelID failed for {}: HRESULT {result}",
            app_id
        );
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

/// Shows a Windows toast on a dedicated thread (required when called from Tokio workers).
pub fn show_toast(app_id: &str, title: &str, body: &str, important: bool) {
    let app_id = app_id.to_string();
    let title = title.to_string();
    let body = body.to_string();

    if std::thread::Builder::new()
        .name("planarclip-toast".into())
        .spawn(move || show_toast_on_thread(&app_id, &title, &body, important))
        .is_err()
    {
        tracing::warn!("Failed to spawn Windows toast thread");
    }
}

fn show_toast_on_thread(app_id: &str, title: &str, body: &str, important: bool) {
    use tauri_winrt_notification::{Scenario, Toast};

    let mut toast = Toast::new(app_id).title(title).text1(body);
    if important {
        toast = toast.scenario(Scenario::Reminder);
    }

    match toast.show() {
        Ok(()) => tracing::info!("Windows toast shown: {body}"),
        Err(error) => tracing::warn!("Windows toast failed ({body}): {error}"),
    }
}
