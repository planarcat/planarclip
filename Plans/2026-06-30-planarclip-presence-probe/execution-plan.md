# PlanarClip 协议级在线探测 — 待执行方案

> 生成时间: 2026-06-30
> 基于讨论: [01-offline-nearby-flicker-bug.md](01-offline-nearby-flicker-bug.md) | [02-presence-vs-machine-online-principle.md](02-presence-vs-machine-online-principle.md)

## 需求概述

PlanarClip 的设备在线/离线判定必须回答：**「对方 PlanarClip 应用角色是否在线」**，而不是「物理机是否开机」或「某个 TCP 端口是否被占用」。

当前实现混用 mDNS、裸 TCP 探针（每 8 秒）和连接事件，导致可信设备在他机已关机后仍在「离线 → 附近 → 正在尝试连接 → 离线」间反复跳动。本方案以**协议级 Presence 探测**替换全部裸 TCP 在线裁决，并修正 `lastSeen` 语义与 auto_connect 触发条件。

## 问题现状

### 已确认的根因

| 问题 | 位置 | 影响 |
|------|------|------|
| 裸 TCP 探针当作在线 | `direct::probe_tcp_reachable*`、`discover_trusted_peers_by_tcp_probe`、`reconcile_lan_devices` | 误判进「附近」 |
| mDNS 陈旧记录无二次校验 | mDNS Added 直接写入 `lan_devices` | 关机后短暂回「附近」 |
| `lastSeen` 每次刷新重置 | `buildDevices` → `lastSeen: new Date()` | 「最近活跃」误导用户 |
| auto_connect 在假阳性上触发 | `maybe_auto_connect_discovered_device` | 「正在尝试连接」闪烁 |

### 不在本方案范围

- 跨网络在线探测（仍只做 LAN）
- 修改 mDNS 服务类型或端口策略（dev/release 隔离保持不变）
- 连接上限、多会话等（见 `2026-06-26-multi-connection-sessions`，已锁定）

## 技术决策

| 决策项 | 选择 | 理由 | 来源轮次 |
|--------|------|------|----------|
| 在线判定标准 | PlanarClip 协议 Presence 回复 | 区分「端口开放」与「PlanarClip 角色在线」 | 第 2 轮 |
| 探测消息 | 新增 `PresenceQuery` / `PresenceReply` | 不触发连接 UI，无副作用 | 第 2 轮 |
| 裸 TCP 探针 | **全部退役** | 是反复误判的直接来源 | 第 1、2 轮 |
| mDNS 角色 | 候选发现，非最终裁决 | 保留零配置发现优势，reconcile 用协议探测兜底 | 第 2 轮 |
| 可信设备校验 | 探测成功时 `peer_id` 必须匹配 | 防止同 IP 换主后张冠李戴 | 第 2 轮 |
| 服务类型校验 | `service_profile` 字段（dev/release） | 避免 dev 实例误判为 release 对等体 | 第 2 轮 |
| 向后兼容 | 旧版不回复 Presence → 视为离线 | 不 fallback 裸 TCP，否则问题依旧 | 第 2 轮 |
| `lastSeen` | 后端 `last_presence_at`，仅探测成功更新 | 语义准确 | 第 2 轮 |
| 离线冷却 | 可选 30s，`peer_offline` 后抑制回「附近」 | 减少网络抖动闪烁 | 第 2 轮 |
| auto_connect 触发 | mDNS Added **且** 协议探测确认 | 避免假阳性连接尝试 | 第 2 轮 |

## 架构设计

### 目标数据流

```text
┌─────────────────────────────────────────────────────────────┐
│ 前端 UI（设备页 / 连接概览）                                  │
│  - 附近 / 离线 分区依据 discoveredOnLan + last_presence_at   │
└───────────────────────────┬─────────────────────────────────┘
                            │ lan-devices-changed（含 last_presence_at）
┌───────────────────────────▼─────────────────────────────────┐
│ lib.rs — lan_devices 维护                                     │
│  - mDNS Added/Removed → 候选增删                              │
│  - refresh_lan_presence（8s）→ 协议探测 reconcile             │
│  - peer_still_reachable → 协议探测                            │
└───────────┬─────────────────────────────┬─────────────────────┘
            │                             │
┌───────────▼──────────┐      ┌───────────▼─────────────────────┐
│ discovery (mDNS)     │      │ direct — probe_planarclip_presence │
│ 候选发现，非最终裁决  │      │ PresenceQuery → PresenceReply     │
└──────────────────────┘      └───────────┬─────────────────────┘
                                          │
                              ┌───────────▼─────────────────────┐
                              │ run_listener                     │
                              │ 第一帧分流：                      │
                              │  PresenceQuery → 静默回复        │
                              │  ConnectRequest → 现有连接流程   │
                              └─────────────────────────────────┘
```

### 协议扩展

在 `network/protocol.rs` 的 `HandshakeMessage` 中新增：

```rust
#[serde(rename = "presence_query")]
PresenceQuery {
    querier_peer_id: String,
},

#[serde(rename = "presence_reply")]
PresenceReply {
    peer_id: String,
    device_name: String,
    service_profile: String,  // "dev" | "release"
},
```

`service_profile` 取值与 `app_profile` 中 `MDNS_SERVICE_TYPE` / build profile 一致。

### 探测 API（Rust）

新增于 `network/direct.rs`：

```rust
pub struct PresenceProbeResult {
    pub peer_id: String,
    pub device_name: String,
    pub service_profile: String,
    pub port: u16,
}

/// Returns Some when the remote speaks PlanarClip presence protocol.
pub async fn probe_planarclip_presence(
    ip: &str,
    ports: &[u16],
    expected_peer_id: Option<&str>,
    timeout: Duration,
) -> Option<PresenceProbeResult>
```

逻辑：

1. 对 `ports`（19877/19876 候选）依次尝试
2. TCP connect → 写 `PresenceQuery` → 读第一帧
3. 必须是 `PresenceReply`；若提供 `expected_peer_id` 则必须匹配
4. `service_profile` 与本地 build profile 一致（dev 对 dev，release 对 release）
5. 任何协议错误、超时、不匹配 → `None`

### Listener 分流

`run_listener` 中，accept 后：

1. 读第一帧（带短 timeout）
2. `PresenceQuery` → 写 `PresenceReply`，shutdown，**不** send `ListenerEvent::Incoming`
3. `ConnectRequest` → 现有 `read_connect_request` 逻辑
4. 其他 / 超时 / 半包 → debug 日志，关闭（保留 `is_likely_probe_disconnect` 语义）

### lan_devices 数据结构扩展

```rust
pub struct LanDevice {
    // ...existing fields...
    #[serde(default)]
    pub last_presence_at: Option<i64>,  // Unix ms，协议探测成功时更新
}
```

前端 `LanDevicePayload` / `Device` 同步增加 `lastPresenceAt`，`buildDevices` 用此后端字段渲染「最近活跃」，删除 `lastSeen: new Date()` 写法。

## 实现步骤

### Phase 1: 协议与探测核心

- [ ] 在 `protocol.rs` 增加 `PresenceQuery` / `PresenceReply`
- [ ] 在 `direct.rs` 实现 `probe_planarclip_presence` 及单元测试（mock listener 回复 PresenceReply）
- [ ] 在 `run_listener` 实现第一帧分流，Presence 路径静默回复
- [ ] 在 `app_profile.rs` 增加 `service_profile_name()` 辅助函数

### Phase 2: 替换全部裸 TCP 在线裁决

- [ ] `discover_trusted_peers_by_tcp_probe` → 重命名并改为 `discover_trusted_peers_by_presence_probe`，使用协议探测 + `peer_id` 校验
- [ ] `reconcile_lan_devices`：prune/保留/port 更新均基于 `probe_planarclip_presence`
- [ ] `peer_still_reachable`：改用协议探测（决定「已下线」vs「连接已断开」）
- [ ] `auto_connect::attempt_connect_trusted_peer` 前置探测：改用协议探测
- [ ] 删除或标记 deprecated 的 `probe_tcp_reachable*`（若无其他调用方则直接删除）

### Phase 3: mDNS 与 auto_connect 收口

- [ ] mDNS `ServiceAdded`：可选立即触发一次 presence 探测后再 emit（或依赖 8s reconcile；推荐 Added 时也探测，减少窗口期）
- [ ] `maybe_auto_connect_discovered_device`：仅在 presence 探测已成功（device 已在 lan_devices 且刚验证）时触发；TCP-probe-only 条目不得触发
- [ ] 实现离线冷却：在 `AppState` 或 config 内存中记录 `peer_offline_at`，冷却期内不因单次 presence 成功将设备从离线拉回附近

### Phase 4: 前端语义修正

- [ ] `LanDevicePayload` / `Device` 增加 `lastPresenceAt`
- [ ] `buildDevices`：使用后端 `last_presence_at`，移除 `lastSeen: new Date()`
- [ ] `DevicesPanel` / `DevicesPage`：「最近活跃 / 最近在线」基于 `lastPresenceAt`
- [ ] 确认 `categorizeDevices` 逻辑不变（仍依赖 `discoveredOnLan`，但后端保证其语义 = 协议 presence 确认）

### Phase 5: 测试与验收

- [ ] Rust 单元测试：`probe_planarclip_presence` 正/负例（正确 peer_id、错误 peer_id、非 PlanarClip 端口、超时）
- [ ] Rust 集成测试：listener PresenceQuery 不触发 Incoming 事件
- [ ] 双机验收场景：
  - [ ] A 连 B 后 B 关机 → A 稳定显示离线，**不**回附近
  - [ ] B 关机 5 分钟内 → 无「正在尝试连接」闪烁
  - [ ] B 重新打开 PlanarClip → 正确回附近并可连接
  - [ ] 同 IP 不同 peer_id（模拟 DHCP 换主）→ 不误认为原设备在线
- [ ] dev/release 交叉：dev 实例不应把 release 实例或随机 TCP 服务判为在线

## 关键依赖

- 现有帧读写：`read_frame` / `write_frame`（`network/direct.rs`）
- 现有 mDNS：`network/discovery.rs`（仅候选发现，逻辑不改服务类型）
- 前端设备合并：`src/app/utils/device.ts`
- 无新外部 crate

## 风险与注意事项

| 风险 | 级别 | 缓解 |
|------|------|------|
| 双端需同步升级 Presence 协议 | 中 | 旧版对 PresenceQuery 无回复 → 探测失败 → 显示离线；联调时需双机同版本 |
| 8s reconcile + 2s 探测超时可能略增 CPU/网络 | 低 | 仅对已有 lan_devices + trusted last_ip 探测，数量通常 ≤5 |
| 离线冷却可能延迟「重新上线」显示 | 低 | 冷却 30s 可配置；mDNS Added + presence 成功可 bypass 冷却 |
| mDNS 与 presence 双重探测 | 低 | Added 时一次 presence 即可，reconcile 作兜底 |

## 验收标准

1. 他机关机且 PlanarClip 未运行后，本机 **10 分钟内** 不出现「附近 → 尝试连接 → 离线」循环
2. 「最近活跃」仅在协议 presence 成功后的时间范围内更新，关机后不再刷新为「N 秒前」
3. 连接断开文案：`peer_still_reachable` 为 false 时显示「已下线」，为 true 时显示「已断开连接」
4. 日志中不再出现 `via TCP probe (mDNS miss)` 类裸 TCP 成功记录，改为 `via presence probe`

## 参考讨论

- [01-offline-nearby-flicker-bug.md](01-offline-nearby-flicker-bug.md) — 问题现象与根因分析
- [02-presence-vs-machine-online-principle.md](02-presence-vs-machine-online-principle.md) — 产品原则与 Presence 协议设计
- 关联历史方案：[2026-06-05-lan-mdns-auto-discovery](../2026-06-05-lan-mdns-auto-discovery/execution-plan.md)（mDNS 发现基线，本方案在其上补在线裁决层）

## 执行顺序建议

优先 **Phase 1 → Phase 2**（后端探测替换），可独立验证日志与 lan_devices 行为；再 **Phase 3 → Phase 4**（auto_connect 与 UI）；最后 **Phase 5** 双机验收。

确认本方案后，说「开始执行」或「按方案做」即可进入开发；说「锁定」则归档本主题。
