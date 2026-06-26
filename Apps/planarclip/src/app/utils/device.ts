import { DEFAULT_DEVICE_NAME } from "../constants/theme";
import type { ConnectedPeer, Device, DeviceBuckets, LanDevicePayload, OS, TrustedPeerPayload } from "../types";

const TRUSTED_PEER_FALLBACK_PORT = import.meta.env.DEV ? 19877 : 19876;

/** IPv6 hosts need brackets when shown with a port, e.g. `[::1]:19876`. */
export function formatDeviceAddress(host: string, port?: number) {
  const trimmed = host.trim();
  if (!trimmed) {
    return "";
  }
  if (port == null) {
    return trimmed;
  }

  const formattedHost = trimmed.includes(":") ? `[${trimmed}]` : trimmed;
  return `${formattedHost}:${port}`;
}

export function inferOs(name: string): OS {
  return /mac|iphone|ipad|ios/i.test(name) ? "macos" : "windows";
}

export function areLanDevicesEqual(left: LanDevicePayload[], right: LanDevicePayload[]) {
  if (left.length !== right.length) {
    return false;
  }

  const sortKey = (device: LanDevicePayload) => device.peer_id;
  const sortedLeft = [...left].sort((a, b) => sortKey(a).localeCompare(sortKey(b)));
  const sortedRight = [...right].sort((a, b) => sortKey(a).localeCompare(sortKey(b)));

  return sortedLeft.every((device, index) => {
    const other = sortedRight[index];
    return (
      device.peer_id === other.peer_id &&
      device.name === other.name &&
      device.ip === other.ip &&
      device.host_name === other.host_name &&
      device.port === other.port
    );
  });
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
      address: formatDeviceAddress(device.ip, device.port),
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
      address: lastIp ? formatDeviceAddress(lastIp, TRUSTED_PEER_FALLBACK_PORT) : "等待对方上线",
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
