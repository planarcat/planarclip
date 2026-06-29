# 未执行 / 遗留事项

> 生成时间: 2026-06-27（Phase 2 执行后更新）
> 最后更新: 2026-06-29（补充同步图片「粘贴为文件」扩展）
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
| 同步图片「粘贴为文件」 | 已实现 | 接收端写 CF_DIB + staging PNG + CF_HDROP | Win 双机联调 |

### 🔮 计划在未来版本加入

| 事项 | 计划版本/Phase | 参考讨论 |
|:---|:---|:---|
| 同步图片接收端支持文件夹粘贴（CF_HDROP） | Phase 1 增强 / 与 Phase 3 staging 共用 | 见下方 § 同步图片粘贴为文件 |
| Phase 3: Windows 文件同步 | 已实现待联调 | execution-plan.md |
| Phase 4: macOS 文件 + 百分比进度条 | Phase 4 | execution-plan.md |
| Phase 5: 多连接图片/文件多播 | Phase 5 | execution-plan.md |
| 后端多路 TCP 会话 | multi-connection | UNEXECUTED 继承 |

---

## 同步图片「粘贴为文件」扩展

> 来源: 2026-06-29 双机联调反馈（截图同步后在 Cursor 聊天可贴图，但资源管理器「粘贴」灰色）

### 现象（当前行为，非 bug）

| 粘贴目标 | 表现 | 原因 |
|:---|:---|:---|
| Cursor / 微信 / 支持图片的编辑器 | ✅ 可粘贴图片 | 读取 `CF_DIB` 或 PNG 格式 |
| 资源管理器文件夹（右键 → 粘贴） | ❌ 灰色不可点 | 需要 `CF_HDROP`（磁盘文件路径列表） |
| 浏览器地址栏 / 纯文本搜索框 | ❌ 不能贴图；部分控件出现 `image.png` 占位文字 | 仅接受文本；对纯图片剪贴板的降级处理 |

当前接收端写入逻辑（`platform/windows/clipboard.rs::write_image`）**只写内存位图**：

- 主格式：`CF_DIB`
- 附加：`PNG` 自定义格式（可选）

与 Win+Shift+S 截图复制行为一致：**能贴到认图片的应用，不能贴成文件**。

### 目标能力

用户从对端收到同步截图后，除现有「在聊天/编辑器里 Ctrl+V 贴图」外，还应能：

1. 在资源管理器目标文件夹 **右键 → 粘贴**，得到本地 `.png` 文件（如 `planarclip-sync-{短 hash}.png`）
2. **同时保留** `CF_DIB` / PNG，避免破坏 Cursor 等应用的直接贴图体验

纯文本输入框（地址栏、单行 search）**不在本能力范围**——即使写入 `CF_HDROP`，这些控件通常也只粘贴路径文字，产品层可接受。

### 建议实现（Windows 优先）

```
收到 PNG（finalize_received_image）
  → 写入 staging 临时文件（与 Phase 3 共用目录）
  → 剪贴板同时放置：
       CF_DIB + PNG（现有，供图片感知应用）
       CF_HDROP 指向 staging 文件（供资源管理器）
  → 可选：注册 FileGroupDescriptor + FileContents 虚拟文件（部分旧应用兼容，非 MVP 必须）
```

| 步骤 | 改动点 | 说明 |
|:---|:---|:---|
| 1 | 新增 `storage/staging.rs`（或 Phase 3 提前落地） | `{app_data}/staging/`，按 hash 命名，定期 GC |
| 2 | 扩展 `platform/windows/clipboard.rs::write_image` | 落盘 PNG 后 `set_without_clear(CF_HDROP, ...)` |
| 3 | `ClipboardMonitor` 回环抑制 | 文件路径写入不应再次触发「本地复制文件」同步（待 Phase 3 一并设计） |
| 4 | macOS（Phase 4） | `NSPasteboard` 写 `fileURL` + 图片数据，与 Win 语义对齐 |
| 5 | 联调验收 | Win↔Win：收到截图 → 文件夹粘贴得 `.png`；Cursor 仍可贴图 |

### 与 Phase 3 文件同步的关系

| 维度 | 本扩展（图片接收写盘） | Phase 3 文件同步 |
|:---|:---|:---|
| 触发 | 对端传来 **图片像素** | 本机复制 **已有文件路径** |
| 发送端 | 已有（PNG inline / 分块） | 需读 `CF_HDROP` 并传字节 |
| 接收端写剪贴板 | 临时 PNG + HDROP | 组装后 staging 文件 + HDROP |
| 共用基础设施 | `staging/` 目录、HDROP 写入、GC | 同左 |

建议：**staging 与 HDROP 写入 helper 在 Phase 3 一并抽象**，本扩展作为 Phase 3 的前置子任务或 Phase 1 增强 patch 交付。

### 验收清单

- [ ] Win 收到同步截图后，资源管理器目标文件夹右键「粘贴」可用，生成 `.png` 文件
- [ ] 同上场景，Cursor / 画图 / 微信等仍可 Ctrl+V 贴图
- [ ] 临时文件不会无限增长（GC 或 hash 去重复用）
- [ ] 大图（512 KB–5 MB 分块）接收完成后同样支持文件夹粘贴
- [ ] macOS 对等能力（Phase 4 或独立 patch）

---

## 🗑️ 已放弃事项（仅记录，不传递）

（无）
