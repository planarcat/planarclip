# 图片与文件剪贴板同步 — 待执行方案



> 生成时间: 2026-06-27  

> 最后更新: 2026-06-29  

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

| 传输进度 UX | Phase 4 百分比进度条 + 完成态 5s 停留 | 用户确认 | 第 2 轮 |

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

- [x] Win 双机：截图互传（2026-06-29）

- [x] **增强**：接收端同步图片支持资源管理器「粘贴为文件」（`CF_HDROP` + staging，与 Phase 3 共用基础设施）

- [x] Win 双机：同步截图 → 资源管理器粘贴为 `.png`（2026-06-29）



### Phase 2：大图分块



- [x] `direct.rs` 新增 `FRAME_BINARY` + `BinaryChunk` 编解码

- [x] `SignalMessage`：ImageBegin / ImageEnd / TransferAck / TransferCancel

- [x] `sync/transfer.rs`：发送窗口、超时、取消

- [x] 接收端流式组装 + 全量 hash 校验

- [x] 超 5MB 拒绝 + 中文通知/日志

- [x] 512 KB–5 MB 图片双机联调（2026-06-29）



### Phase 3：Windows 文件同步



- [x] `platform/windows/clipboard.rs`：CF_HDROP 读写

- [x] `ClipboardSnapshot::FileList` + 发送端分块传输

- [x] `storage/staging.rs` 临时目录与 GC

- [x] 接收端：组装 → 写剪贴板文件列表

- [x] `AppConfig`：`sync_files`、大小上限（沿用 `max_file_bytes`）

- [x] 设置页文件开关

- [x] **增强**：同步图片接收端支持资源管理器「粘贴为文件」（`CF_HDROP` + staging）

- [x] Win 双机联调：**发送端 → 接收端单文件传输**（2026-06-29，`windows279.zip`，含对端拒收中文 reason）

- [x] Win 双机联调：**多文件批次发送**（2026-06-29，13 个文件共 122.5 MB，历史摘要「CC-Switch…等 13 个文件」）

- [x] Win 双机联调：**接收端资源管理器粘贴**（2026-06-29，对端 Downloads 文件夹 Ctrl+V 落盘）

- [x] Win 双机联调：关文件同步时文件名文本回退（2026-06-29，`peer:handled`）



### Phase 4：macOS 文件 + 体验



- [x] `platform/macos/clipboard.rs`：NSPasteboard 文件 URL 读写（`objc2-app-kit`）

- [x] 多文件 batch（`batch_id` / `FileBatchEnd`，Phase 3 已在 Win 验证）

- [x] 前端传输进度（**百分比进度条**；方案见 [03-transfer-progress-design.md](03-transfer-progress-design.md)）

- [x] 剪贴板历史：文件条目（方案见 [04-clipboard-history-file-entries-design.md](04-clipboard-history-file-entries-design.md)）

- [x] **体验打磨**：历史缩略图 data URL 加载、破图回退、`FileClipPreview` 信息垂直居中

- [x] **失败文案**：他机引起失败时展示 `文件同步失败，{对端整句}`（本机问题仍用各自文案）

- [ ] macOS 双机联调：Finder 复制文件 → 同步 → 对端粘贴

- [ ] Win↔Mac 文件交叉测试



### Phase 5：多连接集成



- [x] 依赖 [multi-connection-sessions](../2026-06-26-multi-connection-sessions/execution-plan.md) Phase 1–3

- [x] 图片/文件事件向所有已连接 peer 多播（`ConnectionRegistry` + `SyncEngine` 多播）

- [x] 每 peer 独立 transfer 状态或共享 staging 单次写入（接收端单次写入剪贴板，各 peer 独立发送）

- [x] 全局 2 活跃 transfer 并发限制（`TransferSlotLimiter`）

- [x] 前端多 connectedPeers、`disconnect_peer`、移除单连接切换弹层

- [ ] 5 台联调验收（多连接 + 大文件并发限制）



## 关键依赖



- Rust：`arboard`、`image`、`uuid`、`windows-sys`（已有）

- Rust（新增）：`objc2`、`objc2-foundation`、`objc2-app-kit`（macOS 文件列表）

- 前端：设置页、`ClipTypeIcon`、历史列表（已有类型占位）



## 参考讨论



- [01-image-file-sync-design.md](01-image-file-sync-design.md) — 完整方案

- [2026-05-26-cross-platform-clipboard-sync/07-foreseeable-challenges.md](../2026-05-26-cross-platform-clipboard-sync/07-foreseeable-challenges.md) — 跨平台剪贴板风险

- [2026-06-26-multi-connection-sessions/execution-plan.md](../2026-06-26-multi-connection-sessions/execution-plan.md) — 多连接依赖

- [03-transfer-progress-design.md](03-transfer-progress-design.md) — 百分比进度条（复用右下角状态卡片）

- [04-clipboard-history-file-entries-design.md](04-clipboard-history-file-entries-design.md) — 剪贴板历史文件条目增强

