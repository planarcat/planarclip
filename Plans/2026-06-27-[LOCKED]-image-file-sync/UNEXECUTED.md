# 未执行 / 遗留事项

> 生成时间: 2026-06-27
> 最后更新: 2026-06-30
> 来源主题: 2026-06-27-image-file-sync
> 说明: 本文档记录当前主题执行后仍未完成、未处理或计划未来处理的事项

详细任务与勾选状态见 [execution-plan.md](execution-plan.md)；已验证结果见 [COMPLETED.md](COMPLETED.md)。

---

## ⏳ 仍待处理的遗留事项

### ❌ 执行失败

（无）

### ⚠️ 未开发完成 / 完成度不足

| 事项 | 当前状态 | 差距描述 | 后续建议 |
|:---|:---|:---|:---|
| macOS 图片双机联调 | 代码已完成，Win 端已验证 | macOS 端图片复制/粘贴未在真机验证 | 在 Mac 上与 Win 对测截图互传 |
| macOS 文件同步双机联调 | `platform/macos/clipboard.rs` 已接入 | Finder 复制 → 同步 → 对端粘贴未验证 | 需在 Mac 环境双机联调 |
| Win↔Mac 文件交叉测试 | Phase 4 最后一项 checklist 未勾选 | 跨平台文件同步全链路未测 | Win 发 Mac 收、Mac 发 Win 收各测一轮 |
| 5 台多连接联调验收 | Phase 5 多连接代码已落地 | 连 2–5 台多播、单台 `disconnect_peer` 不影响其他台、全局 2 路大文件并发未实测 | 准备 2–5 台设备做文本/图片/文件多播验收 |

### 📅 被移出本次开发节奏

（无）

### 🔮 计划在未来版本加入

| 事项 | 计划版本/Phase | 参考讨论 |
|:---|:---|:---|
| macOS 历史缩略图 Shell 预览 | Phase 4 可选（P2） | [04-clipboard-history-file-entries-design.md](04-clipboard-history-file-entries-design.md) §10 |
| 多连接场景每 peer 独立进度条 | Phase 5 可选 | [03-transfer-progress-design.md](03-transfer-progress-design.md) |

---

## 🗑️ 已放弃事项（仅记录）

（无）
