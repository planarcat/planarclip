# 背景：为何需要「同时连接 5 台」

> 时间: 2026-06-26

## 产品诉求

用户可在 PlanarClip 中**同时**与最多 **5 台**设备保持连接并同步剪贴板；满员时再发起连接应被拒绝，并提示先断开其中一台。

## 当前实现缺口（2026-06-26 代码现状）

### 前端（已就绪）

- `MAX_CONNECTIONS = 5`（`src/app/constants/connection.ts`）
- 设备页 / 侧栏 / 连接弹层：按 `connectedCount >= 5` 禁用新连接
- 文案：`已超出连接上限，请先断开其中一个设备后再连接。`

### 后端（未就绪）

| 模块 | 现状 | 缺口 |
|------|------|------|
| `AppState.connection` | `Option<ConnectionHandle>` 单槽 | 需 `HashMap<peer_id, ConnectionHandle>` 或等价结构 |
| `AppState.connected` | `bool` | 需改为计数或 `connections.len()` |
| `AppState.connected_peer` | `Option<ConnectedPeerPayload>` | 需 `Vec<ConnectedPeerPayload>` |
| `SyncEngine` | 只向一个 handle 发剪贴板 | 需广播到所有活跃连接 |
| `connect_lan` | 新连接覆盖旧连接 | 需追加连接、独立断开 |
| `disconnect` | 断开全部 pending + 唯一会话 | 需支持 `disconnect_peer(peer_id)` |
| `get_connected_peer` | 返回单个 | 需 `get_connected_peers` 列表 |
| `connection-ended` | 单会话结束 | 需按 peer 移除，不影响其他连接 |
| `auto_connect` | `connected == true` 即不再出站 | 需改为 `count < MAX` |
| 前端 `connectedPeer` | 单个 state | 需 `connectedPeers[]` + `buildDevices` 多 connected |

### 风险

在未完成多连接架构前，前端允许「已连 1 台时再连第 2 台」会在后端**替换**现有会话，造成 silently 断开第一台——当前 UI 在 `connectedCount < 5` 时可能开放第二连接按钮（若第一台已 connected 则 count=1，按钮可用）。**短期行为**：实际仍只能保持 1 条 TCP，第二条会顶掉第一条。

## 继承说明

本主题为**方案定稿 + 未开发**状态，已锁定。后续开发请：

1. 新建执行主题或在本主题 UNEXECUTED 基础上开 `2026-xx-xx-multi-connection-sessions-impl`
2. 以本文与 `execution-plan.md` 为范围基准
3. 完成后更新 `COMPLETED.md` 并做双机「5 台同时在线」验收
