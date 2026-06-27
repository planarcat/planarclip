//! Build-profile constants that keep dev and release instances isolated.

/// User-facing application name shown in window title, tray, notifications, etc.
#[cfg(debug_assertions)]
pub const APP_DISPLAY_NAME: &str = "二向贴（开发）";

#[cfg(not(debug_assertions))]
pub const APP_DISPLAY_NAME: &str = "二向贴";

#[cfg(debug_assertions)]
pub const CONFIG_FILE_NAME: &str = "planarclip_config.dev.json";

#[cfg(not(debug_assertions))]
pub const CONFIG_FILE_NAME: &str = "planarclip_config.json";

#[cfg(debug_assertions)]
pub const DEFAULT_TCP_PORT: u16 = 19877;

#[cfg(not(debug_assertions))]
pub const DEFAULT_TCP_PORT: u16 = 19876;

/// Ports to try when probing a familiar peer by last known IP (covers dev/release mismatch).
pub fn tcp_probe_port_candidates(primary: u16) -> Vec<u16> {
    let alternate = if cfg!(debug_assertions) {
        19876_u16
    } else {
        19877_u16
    };
    if alternate == primary {
        vec![primary]
    } else {
        vec![primary, alternate]
    }
}

#[cfg(debug_assertions)]
pub const MDNS_SERVICE_TYPE: &str = "_planarclip-dev._tcp.local.";

#[cfg(not(debug_assertions))]
pub const MDNS_SERVICE_TYPE: &str = "_planarclip._tcp.local.";

/// Legacy mDNS fullname prefix for entries discovered before `service_fullname` was tracked.
pub fn mdns_service_fullname_prefix(device_name: &str) -> String {
    format!("{device_name}{MDNS_SERVICE_TYPE}")
}
