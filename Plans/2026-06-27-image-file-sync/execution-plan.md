# 图片与文件剪贴板同步 — 待执行方案

> 生成时间: 2026-06-27
> 基于讨论: [01-image-file-sync-design.md](01-image-file-sync-design.md)

## 需求概述

在 PlanarClip 现有**文本**局域网同步基础上，增加：

1. **同步图片**：用户复制截图/图片后，已连接设备剪贴板可粘贴同一图片
2. **同步文件**：用户复制文件（资源管理器/Finder）后，对端可粘贴得到相同文件内容

设置页「同步图片」「同步文件」从「暂不支持」变为可用开关；超限时有中文提示，不暴露技术错误。

## 技术决策

| 决策项 | 选择 | 理由 | 来源 |
|--------|------|------|------|
| 图片跨平台格式 | 统一 PNG | Win/Mac 粘贴兼容最好 | 第 1 轮 |
| 小图传输 | JSON inline base64（≤512 KB） | 复用现有 Data 帧，实现快 | 第 1 轮 |
| 大图/文件传输 | BinaryChunk 帧 + 分块 | 避免 JSON/base64 膨胀；帧上限 16MB 足够 | 第 1 轮 |
| 块大小 / 流控 | 256 KB，窗口 8，TransferAck | 与早期 P2P 方案一致 | 第 1 轮 |
| 图片上限 | 5 MB/张 | 继承 Phase 2 规划 | 第 1 轮 |
| 文件上限 | **100 MB/文件**（用户确认），500 MB/批 | 用户确认默认 100 MB | 第 2 轮 |
| 实现顺序 | **先图片，后文件** | 用户确认 | 第 2 轮 |
| 传输进度 UX | Phase 1 起 indeterminate「正在同步…」；百分比进度条后续 | 用户确认 | 第 2 轮 |
| 文件列表读取 | Win CF_HDROP + Mac NSPasteboard URL | arboard 不支持文件列表 | 第 1 轮 |
| 接收端文件落盘 | `{app_data}/staging/` + 写剪贴板路径 | 模拟本地复制文件行为 | 第 1 轮 |
| 剪贴板探测优先级 | 文件 > 图片 > 文本 | 避免带路径复制时只同步文本 | 第 1 轮 |
| 多设备广播 | 复用 SyncEngine，等多连接主题完成后再多播 | 当前单连接可先交付 | 第 1 轮 + UNEXECUTED |

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                     ClipboardMonitor                         │
│  read: platform files → arboard image → arboard text       │
│  write: platform files / arboard image / arboard text        │
└──────────────────────────┬──────────────────────────────────┘
                           │ ClipboardEvent (Text|Image|FileList)
┌──────────────────────────▼──────────────────────────────────┐
│  SyncEngine + TransferManager                                │
│  dedup (blake3) · settings gates · chunk send/recv           │
└──────────────────────────┬──────────────────────────────────┘
                           │ TCP direct.rs
              ┌────────────┴────────────┐
              │ 0x01 JSON SignalMessage  │  metadata / inline
              │ 0x02 BinaryChunk         │  payload chunks
              └─────────────────────────┘
```

## 实现步骤

### Phase 1：图片同步（inline）

- [x] 扩展 `ClipboardSnapshot` / `ClipboardEvent`（Image 变体）
- [x] `ClipboardMonitor`：探测图片（arboard），PNG 规范化（`image` crate）
- [x] 扩展 `SignalMessage::ClipboardImageInline`
- [x] `ConnectionHandle` 发送/接收图片；接收端 `write_image`
- [x] 去重 + 回环抑制适配 Image hash
- [x] `AppConfig`：`sync_images`、`max_file_bytes`（默认 100 MB，供文件 Phase 使用）
- [x] 设置页开关接入；Tauri 命令读写配置
- [x] 剪贴板历史：图片类型摘要
- [x] 「正在同步图片…」状态提示（侧栏 statusMessage）
- [ ] Win/Mac 手工测试：截图互传（需双机/双实例联调）

### Phase 2：大图分块

- [x] `direct.rs` 新增 `FRAME_BINARY` + `BinaryChunk` 编解码
- [x] `SignalMessage`：ImageBegin / ImageEnd / TransferAck / TransferCancel
- [x] `sync/transfer.rs`：发送窗口、超时、取消
- [x] 接收端流式组装 + 全量 hash 校验
- [x] 超 5MB 拒绝 + 中文通知/日志
- [ ] 512 KB–5 MB 图片双机联调

### Phase 3：Windows 文件同步

- [ ] `clipboard/platform/windows.rs`：CF_HDROP 读写
- [ ] `ClipboardSnapshot::FileList` + 发送端单文件分块
- [ ] `storage/staging.rs` 临时目录与 GC
- [ ] 接收端：组装 → 写剪贴板文件列表
- [ ] `AppConfig`：`sync_files`、大小上限
- [ ] 设置页文件开关

### Phase 4：macOS 文件 + 体验

- [ ] `clipboard/platform/macos.rs`：NSPasteboard 文件 URL
- [ ] 多文件 batch（batch_id / FileBatchEnd）
- [ ] 前端传输进度（**百分比进度条**；Phase 1 已做 indeterminate「正在同步…」）
- [ ] 剪贴板历史：文件条目
- [ ] Win↔Mac 文件交叉测试

### Phase 5：多连接集成

- [ ] 依赖 [multi-connection-sessions](../2026-06-26-multi-connection-sessions/execution-plan.md) Phase 1–3
- [ ] 图片/文件事件向所有已连接 peer 多播
- [ ] 每 peer 独立 transfer 状态或共享 staging 单次写入
- [ ] 5 台场景下大文件并发限制（全局 2 活跃 transfer）

## 关键依赖

- Rust：`arboard`、`image`、`uuid`、`windows-sys`（已有）
- Rust（新增）：`objc2`、`objc2-app-kit`（macOS 文件列表）
- 前端：设置页、`ClipTypeIcon`、历史列表（已有类型占位）

## 参考讨论

- [01-image-file-sync-design.md](01-image-file-sync-design.md) — 完整方案
- [2026-05-26-cross-platform-clipboard-sync/07-foreseeable-challenges.md](../2026-05-26-cross-platform-clipboard-sync/07-foreseeable-challenges.md) — 跨平台剪贴板风险
- [2026-06-26-multi-connection-sessions/execution-plan.md](../2026-06-26-multi-connection-sessions/execution-plan.md) — 多连接依赖
