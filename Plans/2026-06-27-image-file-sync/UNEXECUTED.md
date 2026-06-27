# 未执行 / 遗留事项

> 生成时间: 2026-06-27（Phase 2 执行后更新）
> 来源主题: 2026-06-27-image-file-sync
> 继承自: 2026-06-26-multi-connection-sessions/UNEXECUTED.md

---

## 📋 继承事项的处理结果

| 事项 | 来源主题 | 处理结果 | 说明 |
|:---|:---|:---|:---|
| 后端多路 TCP 会话 | multi-connection-sessions | ⏳ 继续传递 | Phase 5 再接入 |
| 剪贴板多播 | multi-connection-sessions | ⏳ 继续传递 | |
| 按设备断开 | multi-connection-sessions | ⏳ 继续传递 | |
| 前端多 connectedPeer | multi-connection-sessions | ⏳ 继续传递 | |
| 5 台联调验收 | multi-connection-sessions | ⏳ 继续传递 | |

---

## ⏳ 继续传递的遗留事项

### ⚠️ 未开发完成 / 完成度不足

| 事项 | 当前状态 | 差距描述 | 后续建议 |
|:---|:---|:---|:---|
| 图片双机联调 | 未测 | Phase 1 inline + Phase 2 分块均已实现 | Win/Mac 截图互传验证 |
| 512 KB–5 MB 大图 | 已实现未测 | 分块协议已落地 | 复制大截图联调 |

### 🔮 计划在未来版本加入

| 事项 | 计划版本/Phase | 参考讨论 |
|:---|:---|:---|
| Phase 3: Windows 文件同步 | Phase 3 | execution-plan.md |
| Phase 4: macOS 文件 + 百分比进度条 | Phase 4 | execution-plan.md |
| Phase 5: 多连接图片/文件多播 | Phase 5 | execution-plan.md |
| 后端多路 TCP 会话 | multi-connection | UNEXECUTED 继承 |

---

## 🗑️ 已放弃事项（仅记录，不传递）

（无）
