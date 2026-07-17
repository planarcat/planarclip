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

/// Unified mDNS service type -- dev and release share it so instances discover
/// each other by peer_id, not by build profile.
pub const MDNS_SERVICE_TYPE: &str = "_planarclip._tcp.local.";

/// Legacy mDNS fullname prefix for entries discovered before `service_fullname` was tracked.
/// The mDNS instance name is the peer id, so the prefix is keyed by peer id.
pub fn mdns_service_fullname_prefix(peer_id: &str) -> String {
    format!("{peer_id}{MDNS_SERVICE_TYPE}")
}

/// Build profile label returned in presence replies (`dev` or `release`).
pub fn service_profile_name() -> &'static str {
    if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    }
}
