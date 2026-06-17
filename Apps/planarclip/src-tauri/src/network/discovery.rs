use std::net::IpAddr;
use tokio::sync::mpsc;

pub use mdns_sd::{ServiceDaemon, ServiceEvent};

const SERVICE_TYPE: &str = "_planarclip._tcp.local.";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanDevice {
    pub name: String,
    pub peer_id: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    Added(LanDevice),
    Removed(LanDevice),
}

pub fn start_discovery(
    device_name: &str,
    peer_id: &str,
    port: u16,
    event_tx: mpsc::UnboundedSender<DiscoveryEvent>,
) -> Result<ServiceDaemon, mdns_sd::Error> {
    let daemon = ServiceDaemon::new()?;

    let local_ip = local_ip_for_mdns();
    let host_name = hostname();

    tracing::info!(
        target: "planarclip::startup",
        device_name = %device_name,
        peer_id = %peer_id,
        host_name = %host_name,
        local_ip = %local_ip,
        port,
        "preparing mdns service info"
    );

    let service_info = mdns_sd::ServiceInfo::new(
        SERVICE_TYPE,
        device_name,
        &host_name,
        &local_ip,
        port,
        &[("peer_id", peer_id), ("device_name", device_name)][..],
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
                    let ip = info.get_addresses().iter().next().map(|a| a.to_string());

                    if let (Some(ip), true) = (ip, !peer_id.is_empty()) {
                        let device = LanDevice {
                            name,
                            peer_id,
                            ip: ip.clone(),
                            port: info.get_port(),
                        };
                        tracing::info!("mDNS discovered: {} at {}:{}", device.name, ip, device.port);
                        let _ = tx.send(DiscoveryEvent::Added(device));
                    }
                }
                ServiceEvent::ServiceRemoved(instance_name, _service_type) => {
                    let device = LanDevice {
                        name: instance_name,
                        peer_id: String::new(),
                        ip: String::new(),
                        port: 0,
                    };
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

fn local_ip_for_mdns() -> String {
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        if let Some((name, ip)) = interfaces.into_iter().find(|(_, ip)| is_preferred_mdns_ip(ip)) {
            tracing::info!(
                target: "planarclip::startup",
                interface = %name,
                selected_ip = %ip,
                "selected mdns interface address"
            );
            return ip.to_string();
        }
    }

    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn is_preferred_mdns_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            let is_benchmark = octets[0] == 198 && matches!(octets[1], 18 | 19);
            !ip.is_loopback() && !ip.is_link_local() && ip.is_private() && !is_benchmark
        }
        IpAddr::V6(_) => false,
    }
}
