use tokio::sync::mpsc;

pub use mdns_sd::{ServiceDaemon, ServiceEvent};

const SERVICE_TYPE: &str = "_planarclip._tcp.local.";

/// A LAN device discovered via mDNS.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanDevice {
    pub name: String,
    pub peer_id: String,
    pub ip: String,
    pub port: u16,
}

/// Events pushed from the mDNS browser to the coordinator.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    Added(LanDevice),
    Removed(LanDevice),
}

/// Start mDNS advertisement (register our service) and browsing (discover peers).
///
/// Returns immediately after spawning background tasks. Discovered devices are
/// sent through `event_tx`.
pub fn start_discovery(
    device_name: &str,
    peer_id: &str,
    port: u16,
    event_tx: mpsc::UnboundedSender<DiscoveryEvent>,
) -> Result<ServiceDaemon, mdns_sd::Error> {
    let daemon = ServiceDaemon::new()?;

    let local_ip = local_ip_for_mdns();

    let service_info = mdns_sd::ServiceInfo::new(
        SERVICE_TYPE,
        device_name,
        &hostname(),
        &local_ip,
        port,
        &[
            ("peer_id", peer_id),
            ("device_name", device_name),
        ][..],
    )?;

    daemon.register(service_info)?;
    tracing::info!(
        "mDNS registered: {} (_planarclip._tcp) on {}:{}",
        device_name,
        local_ip,
        port
    );

    // Start browsing for peers
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
                    // The removed event only gives us the instance name — we
                    // need to match it to a previously discovered device.
                    // Emit a removed event with just the name; the UI can match.
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
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

fn local_ip_for_mdns() -> String {
    // Try to find a non-loopback IPv4 address.
    // On most home LANs this returns 192.168.x.x or 10.x.x.x.
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}
