# 剪贴板历史文件条目增强 — 设计方案

> 生成时间: 2026-06-29  
> 所属主题: [2026-06-27-image-file-sync](.)  
> 关联 Phase: Phase 4（体验）  
> 状态: 已执行（2026-06-29）

## 1. 背景与目标

Win 文件同步与多文件批次已可用，剪贴板历史能写入文件类事件，但列表里**文件条目与纯文本几乎无差别**——仅一行摘要（如 `report.pdf` 或 `CC-Switch…等 13 个文件`），多文件无法展开查看完整列表，普通文件也无专用视觉样式。

**目标**：在**不改变传输协议**的前提下，让剪贴板历史中的**文件类条目**信息更完整、样式可识别、多文件可浏览；**单文件**在条件允许时展示缩略图或类型图标（见 §4.4–§4.5）。

**非目标（本方案不做）**：

- 历史列表内的传输进度（见 [03-transfer-progress-design.md](03-transfer-progress-design.md)，进度仅在右下角卡片）
- 历史条目跨设备重新下载（历史只反映本机曾同步过的摘要）
- macOS 文件剪贴板读写（见 execution-plan Phase 4 macOS 项；本方案 UI/数据层可先落地，Mac 联调后验收）
- 历史内全文检索、标签、收藏

## 2. 现状

### 2.1 后端（已有）

`lib.rs` → `build_clipboard_history_entry` 对 `ClipboardSnapshot::FileList` 已写入：

| 字段 | 行为 |
|:---|:---|
| `clip_type` | `"file"`；单张图片文件标为 `"image"` |
| `content` | `history_summary_for_files`：单文件文件名；多文件「`{首文件名} 等 {N} 个文件`」；单图文件「`[图片] {W}×{H}`」 |
| `size_label` | 批次总字节，`file_list_size_label` |
| `image_data_url` | 仅**单个图片文件**且本地 `source_path` 可读时生成缩略图 data URL |

接收端 `finalize_received_file` 写入历史时 `source_path` 指向 staging，单图文件理论上可预览。

Win 联调已验证多文件摘要文案（13 文件 / 122.5 MB）。

### 2.2 前端（已有）

| 组件 | 行为 |
|:---|:---|
| `ClipTypeIcon` | `file` 类型显示琥珀色 `File` 图标 |
| `mapClipboardHistory` | 映射 `clip_type`、`size_label`、`image_data_url` |
| `ClipboardPage` | `ClipPreview` / `ClipPreviewGrid` 仅对 `type === "image" && imagePreviewUrl` 显示缩略图；**`file` 类型退化为 `line-clamp` 纯文本** |
| `CopyButton` | 仅 `type === "text"` 显示 |

### 2.3 与设计文档的差距

[01-image-file-sync-design.md](01-image-file-sync-design.md) §7.2 规划了 `file_count`、`thumbnail_ref` 等字段，当前 `ClipboardHistoryEntry` **未包含** `file_count` 与结构化文件名列表。

```text
结论：数据能进历史、摘要文案可用；缺结构化字段 + 文件专用 UI + 可选交互。
```

## 3. 数据模型

### 3.1 扩展 `ClipboardHistoryEntry`（Rust + TS）

在现有字段基础上新增（均 `#[serde(default)]` / 可选，保证旧配置兼容）：

```rust
pub struct ClipboardHistoryEntry {
    // ... 现有字段 ...
    pub file_count: Option<u32>,           // 文件数；单文件为 Some(1)，文本/纯图片为 None
    pub file_names: Option<Vec<String>>,   // 文件名列表（不含路径）；多文件批次用于展开
    pub preview_kind: Option<String>,      // "thumbnail" | "icon"；无预览时为 None
    pub thumbnail_ref: Option<String>,     // 相对路径，如 history_thumbs/{id}.png（见 §4.5）
}
```

- **`image_data_url`**：保留，供现有 `clip_type: image` 条目（截图 / 单图文件）继续使用；新文件缩略图**优先写 `thumbnail_ref`**，避免 JSON 膨胀。
- **`thumbnail_ref`**：相对 `{app_data}/` 的路径；前端通过 Tauri 资源协议或专用命令读取，**不**把 base64 再塞进 config。

前端 `ClipboardHistoryPayload` / `ClipEntry` 同步：

```typescript
type ClipEntry = {
  // ... 现有字段 ...
  fileCount?: number;
  fileNames?: string[];
  previewKind?: "thumbnail" | "icon";
  thumbnailUrl?: string;   // 运行时由 thumbnail_ref 解析，不入库
};
```

### 3.2 写入规则（`build_clipboard_history_entry`）

| 场景 | `clip_type` | `file_count` | `file_names` | `content` |
|:---|:---|:---|:---|:---|
| 单文件 | `file` | `1` | `[file_name]` | 文件名 |
| 多文件批次 | `file` | `N` | 全部文件名（顺序与发送一致） | `{首名} 等 {N} 个文件` |
| 单图片文件 | `image` | `None` | `None` | `[图片] W×H`（保持现状） |
| 关文件同步、仅文件名文本 | `text` | `None` | `None` | 文件名摘要（保持现状，不强行标 file） |

`file_names` 上限建议 **50** 个（与批次上限一致）；超出时 `file_names` 存前 50 个，`content` 仍用摘要。

### 3.3 持久化

- 写入 `planarclip_config.json` 的 `clipboard_history` 数组，与现有条目混存
- 缩略图 PNG 写入 `{app_data}/history_thumbs/{entry_id}.png`，与 config 分离（见 §4.5）
- 旧条目无 `file_count` / `file_names` 时，前端从 `content` 回退展示（不阻断渲染）

## 4. UI 方案

### 4.1 文件条目组件 `FileClipPreview`

新建 `components/common/FileClipPreview.tsx`（或放在 `ClipboardPage` 同目录），供列表/网格共用。

**单文件（`fileCount === 1` 或仅一个 `fileNames`）**

```text
┌─────────────────────────────────────┐
│ ┌──────┐  report.pdf               │
│ │ thumb│  12.4 MB                  │
│ │/icon │                            │
│ └──────┘                            │
└─────────────────────────────────────┘
```

- 左侧预览区（与图片条目同宽）：`previewKind === "thumbnail"` 时显示 `<img>`；`"icon"` 时显示扩展名大图标；均无则 `ClipTypeIcon`
- 主行：文件名，`truncate` + `title` 完整名
- 副行：条目级 `size`（MVP）；二期可加 per-file 大小

**多文件（`fileCount > 1`）**

```text
┌─────────────────────────────────────┐
│ [File图标]  CC-Switch… 等 13 个文件  │
│             122.5 MB · 13 个文件    │
│ ▼ 展开列表                          │
│   · CC-Switch-0.8.0-win.zip  98 MB  │
│   · readme.txt               2 KB   │
│   · ...                             │
└─────────────────────────────────────┘
```

- 默认折叠，点击「展开 / 收起」切换（纯前端 state，不写回后端）
- 展开列表：`fileNames` 逐行显示；暂无单文件大小时只显示文件名
- 超过 10 个文件名时列表区域 `max-h-48 overflow-y-auto`

### 4.2 集成 `ClipboardPage`

`ClipPreview` / `ClipPreviewGrid` 分支：

```tsx
if (clip.type === "file") {
  return <FileClipPreview clip={clip} />;
}
if (clip.type === "image" && clip.imagePreviewUrl) {
  // 保持现有缩略图
}
// text 保持 line-clamp
```

列表视图与网格视图均走同一组件；网格内 `FileClipPreview` 使用紧凑变体（`variant="grid"`）。

### 4.3 视觉规范

- 有缩略图时预览区样式与图片条目一致：`bg-secondary/30` + `rounded-lg border border-border`，`max-h-56 object-contain`
- 仅类型图标时预览区为固定尺寸方框 + 居中图标，不冒充内容预览
- 文案全部中文；「等 N 个文件」与后端 `content` 摘要口径一致
- 不暴露 staging 路径、hash、协议字段

### 4.4 文件缩略图策略

| 场景 | 预览 | 说明 |
|:---|:---|:---|
| 单图片文件（Explorer 复制） | 已有 | `clip_type: image` + `image_data_url`，**保持现状** |
| 单文件 `clip_type: file` + 图片扩展名 | 缩略图 | 与上类似，但走 `thumbnail_ref` + Shell/解码（§4.5） |
| 单文件 PDF / 视频等（Win 有 Shell 预览） | 缩略图 | `IShellItemImageFactory`，`preview_kind: thumbnail` |
| 单文件 `.zip` / `.exe` 等无内容预览 | 类型图标 | `preview_kind: icon`，不读文件内容 |
| 多文件批次 | 默认无图 | 可选：批次内**第一个可预览文件**作代表缩略图（P1.5）；不为每个文件生成 |
| 文件 > 预览大小上限 | 类型图标 | 不读内容（§4.5 上限 **10 MB**） |
| staging 已清理 | 无图 | 仅摘要 + 图标回退 |

**原则**：缩略图是「写历史时的一次性附加」，失败则降级为图标或纯文案，**不阻塞同步主流程**。

### 4.5 缩略图生成与性能

#### 4.5.1 是否要读文件内容？

| 方式 | 是否读全文件 | 适用 |
|:---|:---|:---|
| **Windows Shell 缩略图**（优先） | 通常否 | 单文件；复用资源管理器缓存 / 预览处理器 |
| **扩展名类型图标** | 否 | Shell 失败、超大小上限、`.zip` 等 |
| **自研 `image` 解码**（现状） | **是，整文件 `read`** | 仅作 Shell 不可用时的回退；需受大小上限约束 |

当前 `history_preview_for_files` 对单图文件会 `std::fs::read` 全文件再解码——对大 PNG 有瞬时 IO/内存压力；本方案实施时应**优先 Shell**，并将自研路径限制在 **≤10 MB** 且仅作回退。

#### 4.5.2 性能负担判断

| 维度 | 是否负担 | 说明 |
|:---|:---|:---|
| 触发频率 | 低 | 仅在每次同步写入历史时生成**一次**，非滚动列表时计算 |
| 磁盘读 | Shell：轻；自研解码：重 | 大文件避免自研整文件读 |
| 内存 | Shell：低；自研：峰值与文件大小相关 | 10 MB 上限 + 单条目最多 1 张图 |
| CPU | 一次性 | 解码/缩放只在写历史时；UI 只展示已生成 PNG |
| 持久化 | **需控制** | 禁止大图 base64 进 JSON；用 `history_thumbs/` 目录 |
| UI 滚动 | 几乎无 | 前端只加载已有 `thumbnail_ref` |

结论：在 **Shell 优先 + 大小上限 + 落盘 + blocking 线程** 前提下，对「偶尔同步文件写一条历史」的场景**可接受**；对多文件批次**不为每个文件生成**缩略图。

#### 4.5.3 实现约束（必须）

1. **时机**：在 `build_clipboard_history_entry` 链路中异步生成；**失败返回 `None`**，不 retry 阻塞。
2. **线程**：读盘 / Shell API 放在 `spawn_blocking`，避免占用 tokio 异步 worker。
3. **次数**：单文件最多 1 张；多文件默认 0 张（P1.5 可选 1 张代表图）。
4. **大小上限**：`size_bytes > 10 * 1024 * 1024` 时不尝试内容缩略图，仅类型图标。
5. **输出尺寸**：最长边 **480px**（与现有 `HISTORY_PREVIEW_MAX_EDGE` 一致），PNG 写入 `history_thumbs/{entry_id}.png`。
6. **清理**：清空历史或条目被 truncate 出列表时，删除对应 `history_thumbs` 文件；App 启动可 GC 孤儿缩略图（P2）。
7. **平台**：Win 先落地 Shell；macOS 后续用 `QLThumbnailGenerator`（与 macOS 文件同步 Phase 对齐）。

#### 4.5.4 后端模块（建议）

```text
clipboard/
  history_preview.rs   # 统一入口：preview_for_file(path, file_name, size_bytes) -> PreviewResult
platform/
  windows/thumbnail.rs # Shell 缩略图 / 系统图标
  macos/thumbnail.rs   # 占位，后续实现
storage/
  history_thumbs.rs    # 路径解析、写入、按 entry_id 删除
```

`PreviewResult`：`{ kind: Thumbnail | Icon, ref_path: Option<String> }`；Icon 可用小 PNG 落盘或前端按扩展名映射（二选一，Win 建议落盘以统一 UI）。

#### 4.5.5 与现有 `image_data_url` 的关系

- **`clip_type: image`**（截图、剪贴板图片）：继续 inline `image_data_url`，本方案不强制迁移。
- **`clip_type: file`** 单文件：新逻辑写 `thumbnail_ref` + `preview_kind`。
- 远期可选：截图预览也迁到 `history_thumbs/`，统一减小 config 体积（**非 MVP**）。

## 5. 后端改动清单

| 文件 | 改动 |
|:---|:---|
| `clipboard/types.rs` | `ClipboardHistoryEntry` 增加 `file_count`、`file_names`、`preview_kind`、`thumbnail_ref` |
| `lib.rs` | `build_clipboard_history_entry` 填充新字段；历史写入改为 async + `spawn_blocking` 生成预览（或拆「先写条目、后补 thumb 事件」） |
| `clipboard/file.rs` | `file_names_for_history`；逐步弃用直接写 `image_data_url` 的 file 分支 |
| `clipboard/history_preview.rs` | 新建：预览统一入口 + 大小/次数判断 |
| `clipboard/platform/windows/thumbnail.rs` | 新建：Shell 缩略图与系统图标 |
| `storage/history_thumbs.rs` | 新建：缩略图落盘与 GC |
| `lib.rs` `clear_clipboard_history` | 清空历史时删除 `history_thumbs/` 下对应文件 |

无需新 Tauri 命令读取列表（可选 `resolve_history_thumbnail(id)` 若 asset 协议不便）；仍通过 `get_clipboard_history` / `clipboard-history-changed` 推送。

## 6. 前端改动清单

| 文件 | 改动 |
|:---|:---|
| `types.ts` | `ClipboardHistoryPayload`、`ClipEntry` 扩展字段 |
| `utils/clipboard.ts` | `mapClipboardHistory` 映射新字段；`thumbnail_ref` → 可加载 URL |
| `components/common/FileClipPreview.tsx` | 新建；含缩略图 / 图标 / 纯文案三态 |
| `components/pages/ClipboardPage.tsx` | `ClipPreview` / `ClipPreviewGrid` 接入文件分支 |

## 7. 可选增强（P2，本方案不阻塞 MVP）

| 项 | 说明 |
|:---|:---|
| 重新复制到剪贴板 | Tauri 命令 `restore_clipboard_files(paths)`；校验 staging 文件仍存在 |
| 在文件夹中显示 | Windows `explorer /select,`；需路径有效 |
| 单文件大小 per-row | `file_sizes: Option<Vec<u64>>` 与 `file_names` 对齐 |
| staging 过期占位 | 路径不存在时显示「文件已清理，仅保留摘要」 |
| 文件历史去重 | 同 hash 批次短时间重复只保留一条 |

## 8. 边界与风险

| 场景 | 处理 |
|:---|:---|
| 对端关文件同步，只收到文件名文本 | `clip_type: text`，不走 `FileClipPreview` |
| 接收后 staging 被 GC | 历史摘要仍可见；P2 前不提供重新复制 |
| 单图文件复制 | 继续走 `image` 类型 + 缩略图，**不**走 `file` 分支 |
| 旧历史无 `file_names` | 仅展示 `content` 一行，无展开按钮 |
| 配置体积 | `file_names` 只存文件名；缩略图 PNG **不落 JSON** |
| 大文件预览 | >10 MB 仅图标；Shell 失败不整文件读 |
| 写历史卡顿 | 预览在 `spawn_blocking`；超时（如 3s）则放弃缩略图 |

## 9. 验收标准

- [ ] 发送单文件：历史显示文件名 + 预览区（缩略图或类型图标）+ 总大小
- [ ] 单文件 `.pdf` / 图片（Win）：有内容缩略图或图标回退，同步主流程不卡顿
- [ ] 单文件 >10 MB：仅类型图标，不读全文件
- [ ] 发送 13 文件批次：摘要「等 13 个文件」，可展开见完整文件名列表；默认无批次缩略图
- [ ] 接收端同步文件后：历史条目与发送端摘要一致
- [ ] 单张图片文件：仍为 `image` 类型 + 缩略图（回归）
- [ ] 关文件同步时仅文件名：仍为 `text` 类型（回归）
- [ ] 升级后打开旧 `planarclip_config.json`：历史正常加载，无报错
- [ ] 列表 / 网格视图：文件条目样式一致、无布局错位

## 10. 实现顺序建议

1. **数据层**：`ClipboardHistoryEntry` + `file_count` / `file_names` / `preview_kind` / `thumbnail_ref`
2. **缩略图基础设施**：`history_thumbs/` + Win Shell 预览 + `spawn_blocking` + 10 MB 上限
3. **前端映射**：`types.ts` + `mapClipboardHistory` + 缩略图 URL 解析
4. **UI**：`FileClipPreview`（三态预览）+ `ClipboardPage` 接入
5. **打磨**：展开折叠、网格紧凑变体、旧数据回退、清空历史时删 thumb
6. **（可选）P2**：重新复制、打开文件夹、per-file 大小、孤儿 thumb GC、macOS Shell

## 11. 参考

- [01-image-file-sync-design.md](01-image-file-sync-design.md) §7.2 剪贴板历史
- [execution-plan.md](execution-plan.md) Phase 4「剪贴板历史：文件条目」
- 实现入口：`Apps/planarclip/src-tauri/src/lib.rs`（`build_clipboard_history_entry`）
- 前端入口：`Apps/planarclip/src/app/components/pages/ClipboardPage.tsx`
