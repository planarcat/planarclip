# 第 2 轮：在线判定应以 PlanarClip 角色为准，而非物理设备

> 时间: 2026-06-30

## 用户提问

这个在线判定的问题出现很多次了，到现在都没彻底修复好，最重要的判断是，判断在线与离线要判断对方 PlanarClip 这个角色是否在线 / 离线，而不是人家的机器设备。

## 背景与分析

### 原则对齐

用户明确了产品级判定标准：

> **在线 / 离线 = 对方 PlanarClip 应用角色是否在线，不是物理机是否开机、也不是某个 TCP 端口是否被占用。**

这与当前实现存在根本性偏差。现有补丁式修复（调探针间隔、listener 侧识别 probe disconnect 等）都未改变「端口开放 ≈ 在线」这一错误前提。

### 一次有效的「PlanarClip 在线」应满足

1. 能建立 TCP 连接
2. 对方回复的是 **PlanarClip 握手协议帧**（不是别的服务）
3. 回复里的 `peer_id` 与预期设备一致（对可信设备）
4. 服务类型匹配（dev `_planarclip-dev._tcp` vs 正式 `_planarclip._tcp`）

mDNS 只能作为「候选发现」，不能作为最终在线结论；裸 TCP 更不能。

### 为何不能复用 ConnectRequest 做探测

现有 `ConnectRequest` 会触发完整入站连接流程（弹窗、配对、auto_accept 等），**不适合**做周期性在线探测。需要独立的轻量握手，专门回答「PlanarClip 在不在」。

Listener 已有 `is_likely_probe_disconnect` 处理不完整握手，说明历史上已有人尝试用 connect 帧做 reachability probe，但这对被探测方仍有副作用风险，且探测方未校验响应内容。

## 建议与回答

### 推荐方案：协议级 Presence 探测

在 `HandshakeMessage` 中新增：

```rust
PresenceQuery { querier_peer_id: String }
PresenceReply  { peer_id: String, device_name: String, service_profile: String }
```

**Responder（被探测方）** — 在 `run_listener` 读第一帧时分流：

- `PresenceQuery` → 回 `PresenceReply`，关闭连接，**不进入连接流程、不弹 UI**
- `ConnectRequest` → 走现有逻辑

**Prober（探测方）** — 用 `probe_planarclip_presence(ip, port, expected_peer_id)` 替换所有 `probe_tcp_reachable*`：

- 发 `PresenceQuery`
- 收到合法 `PresenceReply` 且 `peer_id` 匹配 → PlanarClip 在线
- 超时 / 协议错误 / peer_id 不符 → 离线
- TCP 能连但无 PlanarClip 回复 → **不算在线**

### 需替换的调用点

| 位置 | 现状 | 改后 |
|------|------|------|
| `discover_trusted_peers_by_tcp_probe` | 端口开放即加入附近 | 协议探测 + peer_id 校验 |
| `reconcile_lan_devices` | 裸 TCP prune/保留 | 协议探测 prune |
| `peer_still_reachable` | 决定「下线」vs「断连」文案 | 协议探测 |
| `auto_connect` 前置探测 | 裸 TCP | 协议探测（避免对非 PlanarClip 发起连接） |

### 附带修正

1. **`lastSeen` 由后端维护 `last_presence_at`**：只有协议探测成功才更新；前端不再每次 `new Date()` 糊弄。
2. **离线冷却（可选）**：判定 `peer_offline` 后 30～60 秒内不因单次探测成功立刻跳回「附近」，减少网络抖动闪烁。
3. **自动连接触发收紧**：只有 **mDNS 新鲜发现 + 协议探测确认** 才触发 auto_connect。

### 预期效果

- 他机关机后：mDNS 可能短暂误报 → reconcile 协议探测失败 → 不会进「附近」
- 同 IP 换了别的设备 → `peer_id` 不匹配 → 不算原设备在线
- 只有对方 **PlanarClip 进程在跑且响应协议** → 才显示附近 / 允许连接

## 本轮要点

- 用户确认：这是结构性问题，需要替换探测方式，不是再调参数
- 核心决策：新增 `PresenceQuery/PresenceReply`，以协议级 presence 作为唯一在线裁决
- mDNS 保留为候选发现；TCP 裸探针全部退役
- 向后兼容：旧版 PlanarClip 不识别 PresenceQuery 时，探测方应视为离线（不 fallback 到裸 TCP）
