# 未完成摘要

> 来源主题: 2026-06-26-multi-connection-sessions
> 锁定时间: 2026-06-26
> 原路径: Plans/2026-06-26-multi-connection-sessions/

## 仍待处理

- 后端多路 TCP 会话 — 锁定时仍为单 ConnectionHandle
- 剪贴板多播 — 锁定时仍为单 handle 发送
- 按设备 disconnect_peer — 锁定时仅全局 disconnect
- 前端多 connectedPeer — 锁定时仍为单一 state
- 5 台联调验收 — 未做
- 连第 2 台可能覆盖第 1 台会话 — 与 5 台并存目标不一致

## 计划未来做

- connectedCount≥1 时禁止新连接 — 可选临时缓解，未采用

## 已放弃（备查）

（无）
