# 未执行 / 遗留事项

> 生成时间: 2026-06-26  
> 来源主题: 2026-06-26-multi-connection-sessions  
> 说明: 本主题**仅完成方案记录**，以下全部为待开发项

---

## 核心未实现

| 事项 | 当前状态 | 后续 |
|------|----------|------|
| 后端多路 TCP 会话 | 单 `Option<ConnectionHandle>` | 按 execution-plan Phase 1–4 |
| 剪贴板多播 | 单 handle 发送 | Phase 3 |
| 按设备断开 | 仅全局 `disconnect` | 新增 `disconnect_peer` |
| 前端多 connectedPeer | 单一 state | Phase 5 |
| 5 台联调验收 | 未做 | Phase 6 |

---

## 与「连接 UX 改造」的衔接

连接流程 9 条需求中 **第 2 条** 的前端与校验文案已完成；**真正同时 5 台**依赖本主题全部 Phase。

在未完成本主题前请注意：

- UI 在已连 1 台时仍可能显示「可连接第 2 台」（因 `connectedCount < 5`）
- 第二次连接可能在后端**覆盖**第一条会话 — 产品行为与「5 台并存」不一致

**临时缓解（可选，未做）**：在后端多连接完成前，前端可暂时在 `connectedCount >= 1` 时禁止新连接（与旧单连接产品一致）。当前**未采用**此缓解，以免与「5 台」目标冲突。

---

## 继承给下一主题

开新主题时请复制本文件 + `execution-plan.md` + `01-background-and-gap.md` 作为范围输入。
