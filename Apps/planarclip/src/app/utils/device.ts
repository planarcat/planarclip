import { DEFAULT_DEVICE_NAME } from "../constants/theme";
import type { ConnectedPeer, Device, DeviceBuckets, LanDevicePayload, OS, TrustedPeerPayload } from "../types";

const TRUSTED_PEER_FALLBACK_PORT = 19876;

export function inferOs(name: string): OS {
  return /mac|iphone|ipad|ios/i.test(name) ? "macos" : "windows";
}

export function createDeviceId(prefix: string, value: string) {
  return `${prefix}:${value}`;
}

function isPlaceholderDeviceName(name?: string) {
  const trimmed = name?.trim();
  return !trimmed || trimmed === DEFAULT_DEVICE_NAME || trimmed.toLowerCase() === "my device";
}

function pickDeviceName(...names: Array<string | undefined>) {
  return names.find((name) => !isPlaceholderDeviceName(name))?.trim() || names.find((name) => name?.trim())?.trim() || DEFAULT_DEVICE_NAME;
}

export function buildDevices(
  lanDevices: LanDevicePayload[],
  connectedPeer: ConnectedPeer | null,
  trustedPeers: TrustedPeerPayload[],
) {
  const deviceMap = new Map<string, Device>();
  const trustedPeerMap = new Map(trustedPeers.map((peer) => [peer.peer_id, peer]));

  lanDevices.forEach((device) => {
    const trustedPeer = trustedPeerMap.get(device.peer_id);
    const isConnected = connectedPeer != null && connectedPeer.peerId === device.peer_id;
    const displayName = pickDeviceName(connectedPeer?.peerId === device.peer_id ? connectedPeer.name : undefined, trustedPeer?.name, device.name);

    deviceMap.set(device.peer_id, {
      id: createDeviceId(trustedPeer ? "trusted" : "lan", device.peer_id),
      name: displayName,
      os: inferOs(displayName),
      host: device.ip,
      hostName: device.host_name || undefined,
      port: device.port,
      peerId: device.peer_id,
      address: `${device.ip}:${device.port}`,
      status: isConnected ? "connected" : "idle",
      lastSeen: new Date(),
      source: trustedPeer ? "trusted" : "discovery",
      isTrusted: Boolean(trustedPeer),
      autoAccept: trustedPeer ? trustedPeer.auto_accept : false,
      discoveredOnLan: true,
      lastIp: trustedPeer?.last_ip ?? null,
    });
  });

  trustedPeers.forEach((peer) => {
    if (deviceMap.has(peer.peer_id)) {
      return;
    }

    const lastIp = peer.last_ip?.trim() || null;
    deviceMap.set(peer.peer_id, {
      id: createDeviceId("trusted", peer.peer_id),
      name: peer.name,
      os: inferOs(peer.name),
      host: lastIp ?? undefined,
      port: lastIp ? TRUSTED_PEER_FALLBACK_PORT : undefined,
      peerId: peer.peer_id,
      address: lastIp ? `${lastIp}:${TRUSTED_PEER_FALLBACK_PORT}` : "等待对方上线",
      status: "offline",
      source: "trusted",
      isTrusted: true,
      autoAccept: peer.auto_accept,
      discoveredOnLan: false,
      lastIp,
    });
  });

  if (connectedPeer) {
    const connectedKey = connectedPeer.peerId ?? connectedPeer.name;
    const hasConnectedDevice = [...deviceMap.values()].some(
      (device) => device.peerId != null && device.peerId === connectedPeer.peerId,
    );

    if (!hasConnectedDevice) {
      deviceMap.set(connectedKey, {
        id: createDeviceId("connected", connectedKey),
        name: connectedPeer.name,
        os: connectedPeer.os,
        peerId: connectedPeer.peerId,
        address: connectedPeer.address,
        status: "connected",
        lastSeen: new Date(),
        source: "connected",
        isTrusted: connectedPeer.peerId ? trustedPeerMap.has(connectedPeer.peerId) : false,
        autoAccept: connectedPeer.peerId ? (trustedPeerMap.get(connectedPeer.peerId)?.auto_accept ?? false) : false,
        discoveredOnLan: false,
      });
    }
  }

  return [...deviceMap.values()].sort((left, right) => {
    if (left.status !== right.status) {
      return left.status === "connected" ? -1 : 1;
    }
    if (left.isTrusted !== right.isTrusted) {
      return left.isTrusted ? -1 : 1;
    }
    return left.name.localeCompare(right.name, "zh-CN");
  });
}

export function isDeviceReachableOnLan(device: Device) {
  return Boolean(device.discoveredOnLan && device.host?.trim() && device.port);
}

/** Groups merged device records into the three device-management sections from design.
 *
 * Relationship rules:
 * - Familiar + offline → offline
 * - Familiar + online (not connected) → nearby (familiar)
 * - Familiar + connected → paired
 * - Stranger + offline → hidden
 * - Stranger + online (not connected) → nearby (stranger)
 * - Stranger + connected → paired (and persisted as familiar after session)
 */
export function categorizeDevices(devices: Device[]): DeviceBuckets {
  const paired: Device[] = [];
  const nearbyFamiliar: Device[] = [];
  const nearbyStranger: Device[] = [];
  const offline: Device[] = [];

  for (const device of devices) {
    if (device.status === "connected") {
      paired.push(device);
      continue;
    }

    if (isDeviceReachableOnLan(device)) {
      if (device.isTrusted) {
        nearbyFamiliar.push(device);
      } else {
        nearbyStranger.push(device);
      }
      continue;
    }

    if (device.isTrusted) {
      offline.push(device);
    }
  }

  return { paired, nearbyFamiliar, nearbyStranger, offline };
}
