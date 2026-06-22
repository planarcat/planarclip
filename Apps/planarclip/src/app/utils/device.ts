import type { ConnectedPeer, Device, LanDevicePayload, OS } from "../types";

export function inferOs(name: string): OS {
  return /mac|iphone|ipad|ios/i.test(name) ? "macos" : "windows";
}

export function createDeviceId(prefix: string, value: string) {
  return `${prefix}:${value}`;
}

export function buildDevices(lanDevices: LanDevicePayload[], connectedPeer: ConnectedPeer | null) {
  const deviceMap = new Map<string, Device>();

  lanDevices.forEach((device) => {
    const isConnected =
      connectedPeer != null &&
      (connectedPeer.peerId === device.peer_id || connectedPeer.name === device.name);

    deviceMap.set(device.peer_id, {
      id: createDeviceId("lan", device.peer_id),
      name: device.name,
      os: inferOs(device.name),
      host: device.ip,
      port: device.port,
      peerId: device.peer_id,
      address: `${device.ip}:${device.port}`,
      status: isConnected ? "connected" : "idle",
      lastSeen: new Date(),
      source: "discovery",
    });
  });

  if (connectedPeer) {
    const hasConnectedDevice = [...deviceMap.values()].some(
      (device) => device.name === connectedPeer.name || device.peerId === connectedPeer.peerId,
    );

    if (!hasConnectedDevice) {
      deviceMap.set(connectedPeer.peerId ?? connectedPeer.name, {
        id: createDeviceId("connected", connectedPeer.peerId ?? connectedPeer.name),
        name: connectedPeer.name,
        os: connectedPeer.os,
        peerId: connectedPeer.peerId,
        address: connectedPeer.address,
        status: "connected",
        lastSeen: new Date(),
        source: "connected",
      });
    }
  }

  return [...deviceMap.values()].sort((left, right) => {
    if (left.status !== right.status) {
      return left.status === "connected" ? -1 : 1;
    }
    return left.name.localeCompare(right.name, "zh-CN");
  });
}
