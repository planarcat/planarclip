# 传输百分比进度条 — 设计方案

> 生成时间: 2026-06-29  
> 所属主题: [2026-06-27-image-file-sync](.)  
> 关联 Phase: Phase 4（体验）  
> 状态: 待执行

## 1. 背景与目标

大图（512 KB–5 MB）与文件（最高 100 MB/个、多文件批次）已走分块传输。当前仅通过 `clipboard-sync-activity` 推送 indeterminate 文案（「正在同步文件…」），用户无法感知剩余时间。

**目标**：在**不改变现有传输协议**的前提下，为分块传输补充**可读的百分比进度**，并复用界面**右下角固定状态卡片**的视觉与布局（与「正在尝试连接」同源）。

**非目标（本方案不做）**：

- 传输速率曲线、剩余秒数估算
- 用户主动取消传输（按钮）
- 多连接场景下每 peer 独立进度条（留 Phase 5）
- 剪贴板历史列表内的逐条进度

## 2. 现状

| 层级 | 现状 |
|:---|:---|
| 后端 | `transfer.rs` 发送/接收均有 `chunk_total` 与已 ACK/已收块集合，**具备算百分比的数据**，但未上报 |
| 事件 | `emit_sync_activity(active, kind, message)` → `clipboard-sync-activity`，payload 仅 `{ active, kind, message }` |
| 前端 | `useConnectionBridge` 监听事件，只更新侧栏 `lastMessage`（`statusMessage`） |
| 右下角卡片 | `ConnectionAttemptCard`：连接握手时显示，**indeterminate 跑马灯**轨道（`.connection-attempt-track` + marquee 动画） |
| 百分比先例 | `IncomingConnectionPrompt` / `PairingModal`：倒计时用 **determinate 填充条**（`width: progress * 100%`） |

结论：轨道样式与卡片壳已存在；缺的是**通用卡片组件 + 带 progress 的事件 + 后端节流上报**。

## 3. UI 方案

### 3.1 复用右下角卡片

将 `ConnectionAttemptCard` 抽象为通用 **`BottomRightStatusCard`**（命名可微调），固定位置与样式不变：

```text
fixed right-6 bottom-6 z-[70] max-w-[320px]
圆角卡片 + 主文案一行 + 底部轨道
```

| 模式 | 轨道表现 | 使用场景 |
|:---|:---|:---|
| **indeterminate** | 现有 `.connection-attempt-flow` 跑马灯 | 小图 inline（≤512 KB）、连接尝试、进度未知 |
| **determinate** | 同 `.connection-attempt-track` 容器，内部改为**静态填充条**（与配对倒计时条一致：`bg-primary` + `width: N%`） | 分块图片 / 单文件 / 多文件批次 |

**主文案示例**（中文，自然语言）：

| 方向 | 文案模板 |
|:---|:---|
| 发送 | `正在发送 {名称}… {N}%` |
| 接收 | `正在接收 {名称}… {N}%` |
| 多文件 | `正在发送 {首个文件名} 等 {M} 个文件… {N}%` |
| 完成 | `文件已同步` / `图片已同步`（短暂显示后收起，与现有一致） |
| 失败 | 沿用现有 `emit_sync_notice` 自然语言错误，**不显示进度条** |

副文案（可选，小字 `text-muted-foreground`）：`12.4 / 122.5 MB`，仅在 determinate 且 `bytes_total` 已知时显示。

### 3.2 显示优先级（互斥）

同一时刻右下角只显示一张卡片，优先级从高到低：

1. **连接尝试** — `pairingStage === "requesting_device"`（保持现有逻辑）
2. **传输进度** — `transferProgress.active === true`
3. （无）

传输进行中时，侧栏 `statusMessage` **仍可同步更新**（兼容现有行为）；右下角卡片负责**可视化进度**，避免用户必须看侧栏文字。

### 3.3 动效与无障碍

- `prefers-reduced-motion`：determinate 模式禁用过渡动画；indeterminate 可保留较慢 marquee（与现 CSS 一致）
- `role="status"` + `aria-live="polite"`；determinate 时 `aria-valuenow` / `aria-valuemin` / `aria-valuemax` 设在轨道上
- 完成态：进度条先填满 100%，约 **600ms** 后 `active: false` 收起卡片

## 4. 事件协议

### 4.1 扩展 `clipboard-sync-activity`（推荐）

在现有 payload 上**增量字段**，避免新事件名与双监听：

```ts
type ClipboardSyncActivityPayload = {
  active: boolean;
  kind: "image" | "file" | "notice";
  message: string;
  /** 0–1，省略或 null 表示 indeterminate */
  progress?: number | null;
  direction?: "send" | "receive";
  /** 当前条目显示名，如 report.zip */
  label?: string;
  bytes_done?: number;
  bytes_total?: number;
  /** 多文件批次：已完成文件数（不含当前） */
  batch_index?: number;
  batch_total?: number;
};
```

**语义**：

| `active` | `progress` | 前端行为 |
|:---|:---|:---|
| `true` | `null` / 省略 | 显示卡片 + indeterminate 轨道 |
| `true` | `0.0–0.99` | 显示卡片 + 百分比填充 |
| `true` | `1.0` | 填满，准备收尾 |
| `false` | — | 隐藏传输卡片；`message` 可写入侧栏 |

向后兼容：旧前端忽略新字段时行为与现在相同（仅文案）。

### 4.2 上报节流

- 后端：**每 150ms 最多 emit 一次** progress，或进度变化 ≥ 1% 时 emit
- 首块与末块（0%、100%）**立即** emit，避免「卡住」感
- 完成 / 失败 / 取消：**立即** emit `active: false`

## 5. 后端进度计算

### 5.1 单条分块传输（图片 / 单文件）

在 `send_image_chunked`、`send_file_with_flow_control` 及接收端 `ingest_chunk` 中：

```text
progress = acked_or_received_chunk_count / chunk_total
```

发送端以 **已 ACK 块数** 为准（反映真实网络进度，而非仅「已发出」）。

接收端以 **`received` 集合大小 / chunk_total** 为准。

### 5.2 多文件批次

按**字节加权**合并为一条进度（用户感知为「这一批复制」）：

```text
bytes_done = sum(已完成文件大小) + 当前文件已传输字节
bytes_total = 批次内所有文件 metadata.len() 之和
progress = bytes_done / bytes_total
```

`label`：首个未完成文件名；`batch_index` / `batch_total` 供文案「等 M 个文件」。

### 5.3 不显示百分比的路径

| 路径 | 处理 |
|:---|:---|
| 图片 inline（≤512 KB） | 保持 indeterminate「正在同步图片…」 |
| 文本同步 | 不显示右下角卡片 |
| 对端 `peer:handled`（关文件同步文件名回退） | 无分块，indeterminate 或极短闪现 |

### 5.4 代码落点（预估）

| 文件 | 改动 |
|:---|:---|
| `network/webrtc.rs` | 扩展 `emit_sync_activity`，增加 progress 参数；在发送/接收回调传入 |
| `sync/transfer.rs` | 发送循环 ACK 后、接收 `ingest_chunk` 后调用 progress 回调（`Fn(f64, u64, u64)`） |
| `sync/engine.rs` | 多文件批次聚合 bytes（若发送入口在此编排） |

## 6. 前端实现要点

### 6.1 组件拆分

```text
components/overlays/
  BottomRightStatusCard.tsx    # 壳：标题 + ProgressTrack
  ProgressTrack.tsx            # mode: 'indeterminate' | 'determinate', value?: number
  ConnectionAttemptCard.tsx    # 薄包装：title + indeterminate
  TransferProgressCard.tsx     # 薄包装：title + determinate + 可选副文案
```

`ProgressTrack` 的 determinate 分支复用 `.connection-attempt-track` 的背景与圆角，填充条样式对齐 `IncomingConnectionPrompt` 第 73–79 行。

### 6.2 状态

在 `App.tsx` 或小型 hook `useTransferProgress`：

```ts
type TransferProgressState = {
  active: boolean;
  message: string;
  progress: number | null;
  label?: string;
  bytesDone?: number;
  bytesTotal?: number;
} | null;
```

`useConnectionBridge` 内扩展 `clipboard-sync-activity` 处理：更新 `transferProgress` + 可选 `setLastMessage`。

### 6.3 渲染

```tsx
{showConnectionAttemptCard ? (
  <ConnectionAttemptCard ... />
) : transferProgress?.active ? (
  <TransferProgressCard ... />
) : null}
```

## 7. 验收标准

- [ ] 发送 50 MB 单文件：右下角显示「正在发送 xxx…」，进度 0%→100% 单调递增，完成后收起
- [ ] 接收同文件：文案为「正在接收」，进度与发送端大致同步（允许 ±1 块误差）
- [ ] 13 文件批次：显示聚合百分比与「等 N 个文件」
- [ ] 512 KB 以下截图：仍为 indeterminate 或不出百分比（与 5.3 一致）
- [ ] 连接尝试进行中：传输卡片**不遮挡**连接卡片
- [ ] 浏览器预览模式（无 Tauri）：无卡片、无报错

## 8. 实现顺序建议

1. **前端**：`ProgressTrack` + `BottomRightStatusCard`，用 mock state 联调 determinate / indeterminate
2. **协议**：扩展 payload + `useConnectionBridge` 解析
3. **后端**：单文件发送 ACK 进度 → 单文件接收 → 大图分块 → 多文件批次
4. **打磨**：节流、完成态延迟收起、MB 副文案

预估：**0.5–1 天**（不含 macOS 与多连接）。

## 9. 参考

- 现有 indeterminate 样式：`Apps/planarclip/src/styles/index.css`（`.connection-attempt-*`）
- 现有 determinate 先例：`IncomingConnectionPrompt.tsx`、`PairingModal.tsx`
- 原设计简述：`01-image-file-sync-design.md` §7.3
- 执行清单：`execution-plan.md` Phase 4「前端传输进度」
