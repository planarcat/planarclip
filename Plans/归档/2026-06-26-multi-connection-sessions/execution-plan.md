# 待执行方案 — 多连接会话（最多 5 台）

> 状态: **未开发**（仅方案）  
> 锁定时间: 2026-06-26  
> 前置: [01-background-and-gap.md](01-background-and-gap.md)

## 目标

- 本机可同时维持最多 **5** 条独立局域网 TCP 直连
- 剪贴板变更**广播**到所有已连接对端
- 可按设备单独断开，不影响其他连接
- 前端设备列表正确显示多台「已连接」
- 与现有连接流程 UX（配对、拒绝、取消守卫）兼容

## 建议实现阶段

### Phase 1: Rust 状态模型

- [ ] `connections: Arc<Mutex<HashMap<String, ConnectionHandle>>>`
- [ ] `connected_peers: Arc<Mutex<Vec<ConnectedPeerPayload>>>`
- [ ] 移除或派生 `connected: bool` → `!connections.is_empty()`
- [ ] `active_connection_count()` → `connections.len()`
- [ ] `store_connected_peer` → upsert by `peer_id`
- [ ] `remove_connected_peer(peer_id)`

### Phase 2: 连接 / 断开 API

- [ ] `connect_lan`：满员返回「已超出连接上限」；成功 append 而非 replace
- [ ] `disconnect()`：断开全部（保留现有 cancel pending 行为）
- [ ] `disconnect_peer(peer_id: String)`：仅关闭指定会话
- [ ] `get_connected_peers()` → `Vec<ConnectedPeerPayload>`
- [ ] `get_status`：`connected` if count > 0

### Phase 3: SyncEngine 多播

- [ ] `SyncEngine` 持有 `Arc<Mutex<HashMap<...>>>`，本地剪贴板事件 foreach 发送
- [ ] 各 `ConnectionHandle` 独立 dedup 或共享 dedup 策略（需设计：避免 A→B→A 环路）

### Phase 4: 连接生命周期事件

- [ ] `connection-established` / `connection-ended` 带 `peer_id`，前端按 peer 增删
- [ ] 单 peer 掉线不影响其他 peer 的 `connected` 状态

### Phase 5: 前端

- [ ] `connectedPeers: ConnectedPeer[]` 替代单一 `connectedPeer`
- [ ] `buildDevices` 支持多台 `status: connected`
- [ ] 设备页「断开」针对单设备调用 `disconnect_peer`
- [ ] `useConnectionBridge` 轮询 / 事件与多 peer 对齐
- [ ] `auto_connect`：仅在 `count < MAX` 时发起

### Phase 6: 验收

- [ ] 5 台同时在线，任意一台复制文本，其余 4 台收到
- [ ] 断开其中 1 台，其余 4 台仍在线
- [ ] 第 6 台连接被拒绝并提示上限
- [ ] 连接中 UX（配对/拒绝/取消）在 multi 模式下回归

## 非目标（本方案不做）

- 跨网络 / 中继连接
- 图片、文件多连接同步（仍仅文本）

## 参考代码位置

- `Apps/planarclip/src-tauri/src/lib.rs` — AppState、connect/disconnect
- `Apps/planarclip/src-tauri/src/sync/engine.rs` — 同步引擎
- `Apps/planarclip/src-tauri/src/network/webrtc.rs` — `connect_direct`、connection-ended
- `Apps/planarclip/src/app/utils/device.ts` — `buildDevices`
- `Apps/planarclip/src/app/hooks/usePairingFlow.ts` — connectedCount
