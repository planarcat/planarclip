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

    let local_ip = local_ip_for_mdns();
    let host_name = hostname();

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
                        let _ = tx.send(DiscoveryEvent::Added(device));
                    }
                }
                ServiceEvent::ServiceRemoved(instance_name, _service_type) => {
                    let device = LanDevice {
                        name: instance_name,
                        peer_id: String::new(),
                        ip: String::new(),
                        host_name: String::new(),
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

fn normalize_host_name(host_name: &str) -> String {
    host_name
        .trim_end_matches('.')
        .trim_end_matches(".local")
        .to_string()
}

fn local_ip_for_mdns() -> String {
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        if let Some((_, ip)) = interfaces
            .iter()
            .find(|(name, ip)| is_preferred_mdns_ip(name, ip))
        {
            return ip.to_string();
        }

        if let Some((_, ip)) = interfaces
            .iter()
            .find(|(name, ip)| is_fallback_ipv4_for_mdns(name, ip))
        {
            return ip.to_string();
        }

        if let Some((_, ip)) = interfaces
            .iter()
            .find(|(name, ip)| is_fallback_ipv6_for_mdns(name, ip))
        {
            return ip.to_string();
        }

        if let Some((_, ip)) = interfaces.iter().find(|(_, ip)| is_private_ipv4(ip)) {
            return ip.to_string();
        }

        if let Some((_, ip)) = interfaces.iter().find(|(_, ip)| is_routable_ipv4(ip)) {
            return ip.to_string();
        }

        if let Some((_, ip)) = interfaces.iter().find(|(_, ip)| is_routable_ipv6(ip)) {
            return ip.to_string();
        }
    }

    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn is_preferred_mdns_ip(interface_name: &str, ip: &IpAddr) -> bool {
    !is_virtual_interface(interface_name) && is_private_ipv4(ip)
}

fn is_fallback_ipv4_for_mdns(interface_name: &str, ip: &IpAddr) -> bool {
    !is_virtual_interface(interface_name) && is_routable_ipv4(ip)
}

fn is_fallback_ipv6_for_mdns(interface_name: &str, ip: &IpAddr) -> bool {
    !is_virtual_interface(interface_name) && is_routable_ipv6(ip)
}

fn is_private_ipv4(ip: &IpAddr) -> bool {
    matches!(ip, IpAddr::V4(v4) if v4.is_private())
}

fn is_routable_ipv4(ip: &IpAddr) -> bool {
    matches!(ip, IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_link_local())
}

fn is_routable_ipv6(ip: &IpAddr) -> bool {
    matches!(ip, IpAddr::V6(v6) if !v6.is_loopback() && !v6.is_unspecified() && !v6.is_unicast_link_local())
}

fn is_virtual_interface(interface_name: &str) -> bool {
    let name = interface_name.to_ascii_lowercase();
    [
        "mihomo",
        "vethernet",
        "hyper-v",
        "wsl",
        "docker",
        "podman",
        "vmware",
        "virtualbox",
        "tailscale",
        "zerotier",
        "loopback",
        "bluetooth",
        "蓝牙",
        "tun",
        "tap",
        "utun",
        "bridge",
        "hamachi",
    ]
    .iter()
    .any(|keyword| name.contains(keyword))
}

