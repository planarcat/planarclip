use std::net::IpAddr;
use tokio::sync::mpsc;

pub use mdns_sd::{ServiceDaemon, ServiceEvent};

const SERVICE_TYPE: &str = "_planarclip._tcp.local.";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanDevice {
    pub name: String,
    pub peer_id: String,
    pub ip: String,
    pub host_name: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    Added(LanDevice),
    Removed(LanDevice),
}

pub fn start_discovery(
    device_name: &str,
    local_peer_id: &str,
    port: u16,
    event_tx: mpsc::UnboundedSender<DiscoveryEvent>,
) -> Result<ServiceDaemon, mdns_sd::Error> {
    let daemon = ServiceDaemon::new()?;

    let interface_snapshot = local_ip_address::list_afinet_netifas()
        .map(|interfaces| {
            interfaces
                .into_iter()
                .map(|(name, ip)| format!("{}={}", name, ip))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let local_ip = local_ip_for_mdns();
    let host_name = hostname();

    // #region debug-point A:discovery-start
    debug_report_lan(
        "A",
        "network/discovery.rs:start_discovery",
        "[DEBUG] discovery startup parameters prepared",
        serde_json::json!({
            "device_name": device_name,
            "local_peer_id": local_peer_id,
            "port": port,
            "host_name": host_name,
            "selected_ip": local_ip,
            "interfaces": interface_snapshot,
        }),
    );
    // #endregion

    let service_info = mdns_sd::ServiceInfo::new(
        SERVICE_TYPE,
        device_name,
        &host_name,
        &local_ip,
        port,
        &[("peer_id", local_peer_id), ("device_name", device_name)][..],
    )?;

    daemon.register(service_info)?;
    tracing::info!(
        "mDNS registered: {} (_planarclip._tcp) on {}:{}",
        device_name,
        local_ip,
        port
    );

    let browse_rx = daemon.browse(SERVICE_TYPE)?;
    let tx = event_tx;
    let local_peer_id = local_peer_id.to_string();

    std::thread::spawn(move || {
        while let Ok(event) = browse_rx.recv() {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let name = info
                        .get_property_val_str("device_name")
                        .unwrap_or(info.get_fullname())
                        .to_string();
                    let peer_id = info
                        .get_property_val_str("peer_id")
                        .unwrap_or("")
                        .to_string();
                    let host_name = normalize_host_name(info.get_hostname());
                    let addresses = info
                        .get_addresses()
                        .iter()
                        .map(|address| address.to_string())
                        .collect::<Vec<_>>();
                    let ip = addresses.first().cloned();
                    let missing_peer_id = peer_id.is_empty();
                    let is_self = peer_id == local_peer_id;

                    // #region debug-point B:service-resolved
                    debug_report_lan(
                        "B",
                        "network/discovery.rs:service-resolved",
                        "[DEBUG] discovery resolved mDNS service",
                        serde_json::json!({
                            "resolved_name": &name,
                            "resolved_peer_id": &peer_id,
                            "host_name": &host_name,
                            "port": info.get_port(),
                            "addresses": &addresses,
                            "selected_ip": &ip,
                            "missing_peer_id": missing_peer_id,
                            "is_self": is_self,
                            "fullname": info.get_fullname(),
                        }),
                    );
                    // #endregion

                    if let (Some(ip), true) = (ip.clone(), !missing_peer_id && !is_self) {
                        let device = LanDevice {
                            name,
                            peer_id,
                            ip: ip.clone(),
                            host_name,
                            port: info.get_port(),
                        };
                        tracing::info!(
                            "mDNS discovered: {} at {}:{} ({})",
                            device.name,
                            ip,
                            device.port,
                            device.host_name
                        );
                        // #region debug-point C:forward-discovery-event
                        debug_report_lan(
                            "C",
                            "network/discovery.rs:forward-discovery-event",
                            "[DEBUG] discovery forwarded resolved device to app",
                            serde_json::json!({
                                "device_name": &device.name,
                                "peer_id": &device.peer_id,
                                "ip": &device.ip,
                                "host_name": &device.host_name,
                                "port": device.port,
                            }),
                        );
                        // #endregion
                        let _ = tx.send(DiscoveryEvent::Added(device));
                    } else {
                        // #region debug-point C:filtered-resolved-service
                        debug_report_lan(
                            "C",
                            "network/discovery.rs:filtered-resolved-service",
                            "[DEBUG] discovery filtered resolved service",
                            serde_json::json!({
                            "selected_ip": &ip,
                            "resolved_peer_id": &peer_id,
                            "missing_peer_id": missing_peer_id,
                            "is_self": is_self,
                        }),
                        );
                        // #endregion
                    }
                }
                ServiceEvent::ServiceRemoved(instance_name, _service_type) => {
                    let device = LanDevice {
                        name: instance_name.clone(),
                        peer_id: String::new(),
                        ip: String::new(),
                        host_name: String::new(),
                        port: 0,
                    };
                        // #region debug-point D:service-removed
                        debug_report_lan(
                            "D",
                            "network/discovery.rs:service-removed",
                            "[DEBUG] discovery observed service removal",
                            serde_json::json!({
                                "instance_name": instance_name,
                            }),
                        );
                        // #endregion
                    let _ = tx.send(DiscoveryEvent::Removed(device));
                }
                _ => {}
            }
        }
        tracing::warn!("mDNS browse receiver closed");
    });

    Ok(daemon)
}

fn hostname() -> String {
    let host = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());
    let host = host.trim_end_matches('.');

    if host.ends_with(".local") {
        format!("{host}.")
    } else {
        format!("{host}.local.")
    }
}

fn normalize_host_name(host_name: &str) -> String {
    host_name
        .trim_end_matches('.')
        .trim_end_matches(".local")
        .to_string()
}

fn local_ip_for_mdns() -> String {
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        if let Some((_, ip)) = interfaces.into_iter().find(|(_, ip)| is_preferred_mdns_ip(ip)) {
            return ip.to_string();
        }
    }

    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn is_preferred_mdns_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => !v4.is_loopback() && !v4.is_link_local(),
        IpAddr::V6(v6) => !v6.is_loopback() && !v6.is_unspecified(),
    }
}

pub(crate) fn debug_report_lan(hypothesis_id: &str, location: &str, msg: &str, data: serde_json::Value) {
    let env_path = std::path::Path::new(".dbg/lan-discovery.env");
    let mut debug_server_url = "http://127.0.0.1:7777/event".to_string();
    let mut session_id = "lan-discovery".to_string();

    if let Ok(contents) = std::fs::read_to_string(env_path) {
        for line in contents.lines() {
            if let Some(value) = line.strip_prefix("DEBUG_SERVER_URL=") {
                debug_server_url = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("DEBUG_SESSION_ID=") {
                session_id = value.trim().to_string();
            }
        }
    }

    let Some((authority, path)) = debug_server_url
        .trim_start_matches("http://")
        .split_once('/')
    else {
        return;
    };

    let payload = serde_json::json!({
        "sessionId": session_id,
        "runId": "pre-fix",
        "hypothesisId": hypothesis_id,
        "location": location,
        "msg": msg,
        "data": data,
        "ts": now_ms(),
    });

    let Ok(body) = serde_json::to_vec(&payload) else {
        return;
    };

    if let Ok(mut stream) = std::net::TcpStream::connect(authority) {
        let request = format!(
            "POST /{} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            path,
            authority,
            body.len()
        );
        let _ = std::io::Write::write_all(&mut stream, request.as_bytes());
        let _ = std::io::Write::write_all(&mut stream, &body);
        let _ = std::io::Write::flush(&mut stream);
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
