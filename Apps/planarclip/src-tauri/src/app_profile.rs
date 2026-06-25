//! Build-profile constants that keep dev and release instances isolated.

#[cfg(debug_assertions)]
pub const CONFIG_FILE_NAME: &str = "planarclip_config.dev.json";

#[cfg(not(debug_assertions))]
pub const CONFIG_FILE_NAME: &str = "planarclip_config.json";

#[cfg(debug_assertions)]
pub const DEFAULT_TCP_PORT: u16 = 19877;

#[cfg(not(debug_assertions))]
pub const DEFAULT_TCP_PORT: u16 = 19876;

#[cfg(debug_assertions)]
pub const MDNS_SERVICE_TYPE: &str = "_planarclip-dev._tcp.local.";

#[cfg(not(debug_assertions))]
pub const MDNS_SERVICE_TYPE: &str = "_planarclip._tcp.local.";

/// Legacy mDNS fullname prefix for entries discovered before `service_fullname` was tracked.
pub fn mdns_service_fullname_prefix(device_name: &str) -> String {
    format!("{device_name}{MDNS_SERVICE_TYPE}")
}
