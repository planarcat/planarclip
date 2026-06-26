# 执行结果确认 — 连接流程 UX 改造（9 条需求）

> 执行完成时间: 2026-06-26  
> 关联文档: 用户 2026-06-26 提出的 9 条连接交互与文案需求  
> 代码范围: `PairingModal`、`IncomingConnectionPrompt`、`usePairingFlow`、`message.ts`、Rust `connect_lan` / `disconnect` / `direct.rs`

## 逐项核查

| # | 需求摘要 | 状态 | 说明 |
|---|----------|------|------|
| 1 | 连接面板：未选设备不显示配对码；等待回应不显示；对方同意后才进入配对 | ✅ 完成 | `PairingModal` 仅 `incoming_pairing` 显示本机码，`awaiting_code` 显示输入框，`requesting_device` 仅状态条 |
| 2 | 最多连 5 台，满员不可再连 | ⚠️ 部分完成 | 前端上限与文案已完成；**后端仍为单 TCP 会话**，`active_connection_count` 仅 0/1，见 [../2026-06-26-multi-connection-sessions/UNEXECUTED.md](../2026-06-26-multi-connection-sessions/UNEXECUTED.md) |
| 3 | 等待对方回应超时 = 拒绝反馈 | ✅ 完成 | `isConnectionRejected` 含 `timeout`；文案统一为「对方拒绝了这次连接。」 |
| 4 | 对方拒绝时关闭连接窗口 | ✅ 完成 | `handleTerminalConnectionFailure` → `resetPairingFlow(true)` + `StatusNotice` |
| 5 | 不提示去看对方配对码 | ✅ 完成 | 改为「对方已同意连接，请输入 6 位配对码。」等 |
| 6 | 连接期间不可重选设备 | ✅ 完成 | `connectionLocked` 禁用列表；`switchPairingTarget` 早退；侧栏 `connecting` 禁用 |
| 7 | 取消连接守卫 | ⚠️ 部分完成 | 前端 `outboundCancelledRef` + 后端 `initiator_abort` / `abort_outbound_handshake`；入站 `incoming_pairing` 关窗仍走 `incomingRequest` 分支，文案误用「超时取消」（见遗留） |
| 8 | 入站等待回应倒计时，超时默认拒绝 | ✅ 完成 | `IncomingConnectionPrompt` 60s 倒计时 + `onTimeout` → `reject_connection` |
| 9 | 文案规范 | ⚠️ 大部分完成 | 见 `message.ts` 常量；个别边界文案与 `CONNECTION.md` 未同步更新 |

## 遗留 / 建议跟进

1. **入站配对关窗文案**：`closePairingModal` 在 `incomingRequest` 存在时一律用 `MSG_SELF_INCOMING_TIMEOUT`，手动关闭 `incoming_pairing` 应区分「主动取消」与「倒计时超时」。
2. **主动方输码阶段倒计时**：`awaiting_code` 倒计时到期仅本地清空输入并提示「配对码已更新」，未订阅对端 `pairing-code-rotated` 事件拉取新码。
3. **CONNECTION.md**：仍为改造前产品描述，需按新流程修订。
4. **双机验收**：9 条需求未做系统化手动回归清单执行记录。

## 与多连接主题关系

第 2 条「5 台同时连接」的后端能力已拆至独立方案主题并锁定归档：  
[../2026-06-26-multi-connection-sessions/](../2026-06-26-multi-connection-sessions/)
