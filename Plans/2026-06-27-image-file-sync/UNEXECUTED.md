# 未执行 / 遗留事项

> 生成时间: 2026-06-27
> 最后更新: 2026-06-29（Win 大图分块、截图粘贴为文件、关文件同步文件名回退已验证）
> 来源主题: 2026-06-27-image-file-sync
> 继承自: 2026-06-26-multi-connection-sessions/UNEXECUTED.md

详细任务与勾选状态见 [execution-plan.md](execution-plan.md)；已验证结果见 [COMPLETED.md](COMPLETED.md)。

---

## 继承事项（继续传递）

| 事项 | 处理结果 |
|:---|:---|
| 后端多路 TCP 会话 | Phase 5 |
| 剪贴板多播 | Phase 5 |
| 按设备断开 | Phase 5 |
| 前端多 connectedPeer | Phase 5 |
| 5 台联调验收 | Phase 5 |

---

## 待联调 / 待验收

| 事项 | 说明 |
|:---|:---|
| 图片 macOS 双机联调 | Win 端 inline / 分块 / 资源管理器粘贴已验证 |

---

## 未开发（后续 Phase）

| 事项 | Phase | 参考 |
|:---|:---|:---|
| macOS 文件同步（NSPasteboard） | 4 | execution-plan.md |
| 百分比进度条 | 4 | [03-transfer-progress-design.md](03-transfer-progress-design.md) |
| 多文件 batch 协议（batch_id / FileBatchEnd） | 4 | 当前多文件批次已可用，正式协议待定 |
| 剪贴板历史文件条目增强 | 4 | execution-plan.md |
| Win↔Mac 文件交叉测试 | 4 | execution-plan.md |
| 多连接图片/文件多播 | 5 | execution-plan.md、multi-connection-sessions |

---

## 已放弃

（无）
