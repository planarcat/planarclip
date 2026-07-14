import { describe, expect, it } from "vitest";

import {
  areLanDevicesEqual,
  buildDevices,
  categorizeDevices,
  createDeviceId,
  formatDeviceAddress,
  inferOs,
  isDeviceReachableOnLan,
} from "../device";
import type {
  ConnectedPeer,
  LanDevicePayload,
  TrustedPeerPayload,
} from "../../types";

// 覆盖 utils/device.ts 中的分类与合并逻辑。
// 该模块承担 UI 三区（已配对 / 附近 / 离线）分区的核心裁决，回归价值最高。

describe("formatDeviceAddress", () => {
  it("IPv4 直接拼接", () => {
    expect(formatDeviceAddress("192.168.0.10", 19876)).toBe("192.168.0.10:19876");
  });

  it("IPv6 host 用中括号包裹", () => {
    expect(formatDeviceAddress("fe80::1", 19876)).toBe("[fe80::1]:19876");
  });

  it("无端口时仅返回 host", () => {
    expect(formatDeviceAddress("192.168.0.10")).toBe("192.168.0.10");
  });

  it("空 host 返回空串", () => {
    expect(formatDeviceAddress("   ", 19876)).toBe("");
  });
});

describe("inferOs", () => {
  it("包含 mac/iphone/ipad/ios 关键字识别为 macos", () => {
    expect(inferOs("Sam's MacBook")).toBe("macos");
    expect(inferOs("iPhone 15")).toBe("macos");
    expect(inferOs("ipad-air")).toBe("macos");
    expect(inferOs("ios-device")).toBe("macos");
  });

  it("其他名称默认识别为 windows", () => {
    expect(inferOs("planarcat-win11")).toBe("windows");
    expect(inferOs("我的设备")).toBe("windows");
  });
});

describe("createDeviceId", () => {
  it("按 `prefix:value` 组合", () => {
    expect(createDeviceId("trusted", "abc123")).toBe("trusted:abc123");
  });
});

describe("areLanDevicesEqual", () => {
  const base: LanDevicePayload = {
    name: "A",
    peer_id: "p1",
    ip: "192.168.0.10",
    host_name: "host-a",
    port: 19876,
    last_presence_at: 1_700_000_000_000,
  };

  it("对象一致时返回 true（顺序无关）", () => {
    const left = [base, { ...base, peer_id: "p2", ip: "192.168.0.11" }];
    const right = [{ ...base, peer_id: "p2", ip: "192.168.0.11" }, base];
    expect(areLanDevicesEqual(left, right)).toBe(true);
  });

  it("长度不同直接判负", () => {
    expect(areLanDevicesEqual([base], [])).toBe(false);
  });

  it("任一字段变化返回 false", () => {
    expect(areLanDevicesEqual([base], [{ ...base, ip: "10.0.0.1" }])).toBe(false);
    expect(areLanDevicesEqual([base], [{ ...base, port: 19877 }])).toBe(false);
    expect(areLanDevicesEqual([base], [{ ...base, last_presence_at: 1 }])).toBe(false);
  });
});

// buildDevices 是设备合并的主入口，测试覆盖三类来源与优先级：
// LAN 发现 + 熟悉设备（trusted）+ 已连接会话（connected）。
describe("buildDevices", () => {
  const lan: LanDevicePayload = {
    name: "planarcat-win11",
    peer_id: "peer-A",
    ip: "192.168.0.10",
    host_name: "planarcat",
    port: 19876,
    last_presence_at: 1_700_000_000_000,
  };
  const trusted: TrustedPeerPayload = {
    name: "planarcat-win11",
    peer_id: "peer-A",
    last_ip: "192.168.0.10",
    auto_accept: true,
  };
  const connected: ConnectedPeer = {
    name: "planarcat-win11",
    peerId: "peer-A",
    address: "192.168.0.10:19876",
    os: "windows",
    source: "lan",
  };

  it("LAN + trusted + connected 三源同 peer 合并为 1 条 connected", () => {
    const devices = buildDevices([lan], [connected], [trusted]);
    expect(devices).toHaveLength(1);
    expect(devices[0].status).toBe("connected");
    expect(devices[0].isTrusted).toBe(true);
    expect(devices[0].autoAccept).toBe(true);
    expect(devices[0].discoveredOnLan).toBe(true);
    expect(devices[0].source).toBe("trusted");
    expect(devices[0].id).toBe("trusted:peer-A");
  });

  it("仅 LAN 存在（陌生设备）标记为 idle 且不可信", () => {
    const devices = buildDevices([lan], [], []);
    expect(devices).toHaveLength(1);
    expect(devices[0].status).toBe("idle");
    expect(devices[0].isTrusted).toBe(false);
    expect(devices[0].id).toBe("lan:peer-A");
  });

  it("仅 trusted 存在时视为离线", () => {
    const devices = buildDevices([], [], [trusted]);
    expect(devices).toHaveLength(1);
    expect(devices[0].status).toBe("offline");
    expect(devices[0].discoveredOnLan).toBe(false);
    expect(devices[0].address).toContain("192.168.0.10");
  });

  it("trusted 无 last_ip 时地址显示为『等待对方上线』", () => {
    const devices = buildDevices([], [], [{ ...trusted, last_ip: null }]);
    expect(devices[0].address).toBe("等待对方上线");
    expect(devices[0].host).toBeUndefined();
  });

  it("connected 但无 LAN / trusted 记录时以 connected 兜底", () => {
    const devices = buildDevices([], [connected], []);
    expect(devices).toHaveLength(1);
    expect(devices[0].status).toBe("connected");
    expect(devices[0].source).toBe("connected");
  });

  it("排序：connected 一定在最前，其他条目集合完整", () => {
    // 说明：buildDevices 的次级排序对 status !== connected 的两条目返回不稳定，
    // 这里只断言：connected 排第一 + 集合成员齐全，不锁死 idle/offline 混排顺序。
    const lanZ: LanDevicePayload = { ...lan, peer_id: "peer-Z", name: "z-device" };
    const trustedM: TrustedPeerPayload = {
      name: "m-device",
      peer_id: "peer-M",
      last_ip: null,
      auto_accept: false,
    };
    const list = buildDevices([lan, lanZ], [connected], [trusted, trustedM]);
    expect(list).toHaveLength(3);
    expect(list[0].peerId).toBe("peer-A");
    expect(list[0].status).toBe("connected");
    const rest = list.slice(1).map((d) => d.peerId).sort();
    expect(rest).toEqual(["peer-M", "peer-Z"]);
  });
});

describe("isDeviceReachableOnLan", () => {
  it("必须同时有 discoveredOnLan / host / port", () => {
    const [lanOnly] = buildDevices(
      [
        {
          name: "x",
          peer_id: "peer-x",
          ip: "192.168.0.10",
          host_name: "x",
          port: 19876,
          last_presence_at: 0,
        },
      ],
      [],
      [],
    );
    expect(isDeviceReachableOnLan(lanOnly)).toBe(true);
  });

  it("trusted 离线设备不可达", () => {
    const [trustedOffline] = buildDevices(
      [],
      [],
      [{ name: "x", peer_id: "peer-x", last_ip: null, auto_accept: false }],
    );
    expect(isDeviceReachableOnLan(trustedOffline)).toBe(false);
  });
});

describe("categorizeDevices", () => {
  it("按 connected / LAN + trusted / LAN 陌生 / trusted 离线 四种情形分区", () => {
    const lanTrusted: LanDevicePayload = {
      name: "trusted-online",
      peer_id: "peer-T",
      ip: "192.168.0.10",
      host_name: "t",
      port: 19876,
      last_presence_at: 0,
    };
    const lanStranger: LanDevicePayload = {
      name: "stranger",
      peer_id: "peer-S",
      ip: "192.168.0.11",
      host_name: "s",
      port: 19876,
      last_presence_at: 0,
    };
    const connectedPeer: ConnectedPeer = {
      name: "connected",
      peerId: "peer-C",
      address: "192.168.0.12:19876",
      os: "windows",
      source: "lan",
    };
    const trustedList: TrustedPeerPayload[] = [
      { name: "trusted-online", peer_id: "peer-T", last_ip: "192.168.0.10", auto_accept: false },
      { name: "trusted-offline", peer_id: "peer-O", last_ip: null, auto_accept: false },
    ];

    const devices = buildDevices([lanTrusted, lanStranger], [connectedPeer], trustedList);
    const buckets = categorizeDevices(devices);

    expect(buckets.paired.map((d) => d.peerId)).toEqual(["peer-C"]);
    expect(buckets.nearbyFamiliar.map((d) => d.peerId)).toEqual(["peer-T"]);
    expect(buckets.nearbyStranger.map((d) => d.peerId)).toEqual(["peer-S"]);
    expect(buckets.offline.map((d) => d.peerId)).toEqual(["peer-O"]);
  });
});