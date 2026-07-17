# SyncEngine 可测性抽象 — 待执行方案

> 生成时间: 2026-07-11
> 基于讨论: [01-背景与目标.md](01-背景与目标.md) | [02-影响面与调用图.md](02-影响面与调用图.md) | [03-trait设计与替换方案.md](03-trait设计与替换方案.md)

## 需求概述

在上一主题《测试基建搭建》落地后，前端 + Rust 已有 116 条测试。但 Rust 侧 **`sync::engine::SyncEngine`** 直接依赖 `ConnectionHandle` / `ConnectionRegistry` / `AppHandle`，导致核心分派逻辑 `broadcast_snapshot`（4 分支 + 2 短路）**无法被单独测试**——它是整仓库最贵、回归成本最高的一段。

本方案以**外科手术**方式在 `SyncEngine` 与外部世界之间插入两个 trait，使得测试能不启动 Tauri、不建 WebSocket、不发协议帧就复算这 7 个场景。`ConnectionHandle` / `webrtc.rs` / 前端零改动。

## 问题现状

### 可测性瓶颈

| 现状 | 位置 | 阻碍 |
|:---|:---|:---|
| `SyncEngine::new` 强依赖 `AppHandle` | `sync/engine.rs:22` | 单测中无 `AppHandle` 可构造 |
| `broadcast_snapshot` 直接调 `ConnectionHandle::send_*` | `sync/engine.rs:87-131` | 无法验证 4 分支分派 |
| `active_handles()` 返回具体类型 `Vec<ConnectionHandle>` | `network/sessions.rs:71` | 测试塞不进桩数据 |

### 影响面（第 2 轮结论）

| 文件 | 改动 | 风险 |
|:---|:---|:---|
| `Apps/src-tauri/src/sync/engine.rs` | 全部重写 133 行 | 中：4 分支等价性 |
| `Apps/src-tauri/src/sync/out.rs` | 新增（trait + 生产/测试实现） | 低 |
| `Apps/src-tauri/src/sync/mod.rs` | `pub mod out;` | 低 |
| `Apps/src-tauri/src/lib.rs:3219` | 构造点新增 3 行 | 低 |
| `Apps/src-tauri/Cargo.toml` | 增 `async-trait = "0.1"` | 低 |
| `network/webrtc.rs` / `network/sessions.rs` | **零改动** | — |
| 前端 / `Apps/src/**` | **零改动** | — |

## 技术决策

| 决策项 | 选择 | 理由 | 来源轮次 |
|:---|:---|:---|:---|
| trait 数量 | 2 个（`ClipboardOut` + `ClipboardOutProvider`） | 调用面与发现面同时可换 | 第 3 轮 |
| trait 位置 | `sync/out.rs` | 与 `engine.rs` 同模块，语义就近 | 第 3 轮 |
| 快照传参 | `Arc<ClipboardSnapshot>` | 4 分支共享；`.await` Send 稳定 | 第 3 轮 |
| AppHandle 处理 | 内收到 `TauriConnectionOut` | SyncEngine 本体脱 Tauri | 第 3 轮 |
| async trait 支持 | `async-trait = "0.1"` | 与 `dyn Trait + Send` 组合稳 | 第 3 轮 |
| 测试实现 | `InMemoryOut` + `StaticOutsProvider` + `RecordedCall` 枚举 | 断言粒度到 4 分支 | 第 3 轮 |
| E2E / 双机 | 冒烟兜底，不做集成脚本 | 上主题决策：短期不做 E2E | 上一主题 |


## 架构设计

### 目录结构变更

```text
Apps/src-tauri/
├── Cargo.toml              # 增：async-trait = "0.1"
└── src/
    ├── lib.rs              # 改：SyncEngine 构造点（+3 行、-1 入参、+1 入参）
    └── sync/
        ├── mod.rs          # 增：pub mod out;
        ├── engine.rs       # 重写：SyncEngine 依赖 provider + Arc<dyn ClipboardOut>
        └── out.rs          # 新增：trait + TauriConnectionOut + RegistryOutProvider
                            #      + #[cfg(test)] test_support (InMemoryOut, StaticOutsProvider)
```

### trait 与实现对照

| 角色 | 生产实现 | 测试实现 |
|:---|:---|:---|
| `ClipboardOut` | `TauriConnectionOut { inner: ConnectionHandle, app: AppHandle }` | `InMemoryOut { supports_chunked, calls }` |
| `ClipboardOutProvider` | `RegistryOutProvider { registry, app }` | `StaticOutsProvider { outs }` |

### 分支等价性锚点（回归必须复算）

| 快照 | 前置条件 | 期望 out 收到 |
|:---|:---|:---|
| `Text("...")` | 任意 | `Snapshot { sync_images }` |
| `Image` 且 `len > INLINE_IMAGE_BYTES` 且 `supports_chunked=true` | 图片同步开 | `ImageAsync { sync_images: true }` |
| `Image` 且 `len > INLINE_IMAGE_BYTES` 且 `supports_chunked=false` | 图片同步开 | `Snapshot`（webrtc 内落 512KB 提示） |
| `Image` 且 `len <= INLINE_IMAGE_BYTES` | 图片同步开 | `Snapshot` |
| `FileList` | 文件同步开 | `FilesAsync { sync_files: true, max_file_bytes }` |
| `FileList` | 文件同步关 | `Snapshot`（webrtc 内不作任何动作） |
| 任意 | `origin=Remote` | 一个 out 也不调用 |
| 任意 | `auto_sync_clipboard=false && !skip_history_merge` | 一个 out 也不调用 |

## 实施阶段（Phase 1–4）

### Phase 1 — trait 与生产包装（约 60 分钟）

**目标**：`sync/out.rs` 落地，`Cargo` 编译通过；旧 `engine.rs` 与 `lib.rs` 暂不改，此时全仓 `cargo check` 也应通过。

任务清单：

1. `Apps/src-tauri/Cargo.toml` 的 `[dependencies]` 增 `async-trait = "0.1"`。
2. 新建 `Apps/src-tauri/src/sync/out.rs`，内容照抄第 3 轮：
   - `pub trait ClipboardOut`
   - `pub trait ClipboardOutProvider`
   - `pub struct TauriConnectionOut` + `impl ClipboardOut`
   - `pub struct RegistryOutProvider` + `impl ClipboardOutProvider`
   - **不含** `#[cfg(test)] mod test_support`（Phase 3 再加）
3. `Apps/src-tauri/src/sync/mod.rs` 增 `pub mod out;`。
4. 根目录跑 `pnpm check`（转发到 `cargo check`）验证编译。

**Phase 1 验收**：
- [ ] `cargo check` 通过，无 warning
- [ ] `sync/out.rs` 存在，导出 4 个符号
- [ ] `engine.rs` / `lib.rs` / `webrtc.rs` 未改动

### Phase 2 — SyncEngine 与构造点替换（约 90 分钟）

**目标**：`SyncEngine` 完全脱离 `AppHandle`/`ConnectionHandle`；`lib.rs` 构造点用 `RegistryOutProvider` 注入；应用能启动、双机能同步。

任务清单：

1. 全量重写 `Apps/src-tauri/src/sync/engine.rs`（照抄第 3 轮完整代码块）。
2. 改写 `Apps/src-tauri/src/lib.rs:3219` 构造点：
   - 加 `use crate::sync::out::RegistryOutProvider;`
   - 构造 `Arc::new(RegistryOutProvider::new(connections_bg.clone(), app_handle_bg.clone()))`
   - `SyncEngine::new` 少传 `app_handle_bg`、多传 `provider`
3. `pnpm check` + `pnpm test:rust`（Phase 3 尚未加新测试，只是确保现有 49 条 Rust 单测全绿）。
4. 手工冒烟（Phase 4 会正式做，这里先粗筛）：`pnpm dev` 单机启动，看托盘、看主窗、复制一段文字是否报错，不追双机。

**Phase 2 验收**：
- [ ] `pnpm check` 通过
- [ ] `pnpm test:rust` 现有 49 条测试全绿（无新用例）
- [ ] `pnpm dev` 单机启动无 panic
- [ ] `SyncEngine` 结构体字段中不再出现 `AppHandle` / `ConnectionHandle` / `ConnectionRegistry`


### Phase 3 — 加测试实现与分派单测（约 100 分钟）

**目标**：`sync/out.rs` 加 `#[cfg(test)] pub(crate) mod test_support`；`sync/engine.rs` 底部加 `#[cfg(test)] mod tests`，覆盖 4 分派分支 + 2 短路 + 3 开关/空列表副条件 = **共 11 个用例**。

任务清单：

1. `sync/out.rs` 末尾加 `#[cfg(test)] pub(crate) mod test_support`（照抄第 3 轮）：
   - `RecordedCall` 枚举
   - `InMemoryOut`
   - `StaticOutsProvider`
2. `sync/engine.rs` 末尾加 `#[cfg(test)] mod tests`，用例列表（11 条）：

   **A. 4 个分派分支**
   - `text_snapshot_goes_to_all_outs_send_snapshot`
   - `small_image_uses_send_snapshot`
   - `large_image_with_chunked_uses_image_async`
   - `large_image_without_chunked_falls_back_to_send_snapshot`
   - `file_list_with_sync_uses_files_async`
   - `file_list_without_sync_falls_back_to_send_snapshot`（此条为分支必备，与上一条对偶）

   **B. 2 个短路**
   - `remote_origin_event_is_skipped`
   - `auto_sync_off_and_not_skip_history_is_skipped`

   **C. 3 个副条件（本轮新增）**
   - `sync_images_off_large_image_falls_back_to_send_snapshot` — 大图 + `sync_images=false`，应落到 `send_snapshot` 而非 `send_image_async`
   - `sync_files_off_file_list_falls_back_to_send_snapshot` — 文件 + `sync_files=false`，应落到 `send_snapshot` 而非 `send_files_async`
   - `empty_outs_list_does_not_panic_and_no_spawn` — provider 返回空 `Vec`，`broadcast_snapshot` 不 panic、不 spawn

3. 每个用例通用套路：
   - `let (tx, rx) = broadcast::channel(4);`
   - `AppConfig::default()` 起手，按需覆盖 4 个开关字段（`sync_images` / `sync_files` / `max_file_bytes` / `auto_sync_clipboard`）
   - `TransferSlotLimiter::new(2)`
   - `Arc::new(StaticOutsProvider { outs: vec![...] })`
   - `tokio::spawn(engine.run())`；`tx.send(event)?`；`tokio::time::sleep(50ms)`
   - 断言 `out.recorded()` 匹配预期
4. 跑 `pnpm test:rust`，从 49 → **60** 全绿（+11）。
5. 跑 `pnpm test`（前后端全量），确认前端 67 条也没被牵连。

**不在本 Phase 覆盖**（属于 network 层职责，留给后续主题）：

- `ConnectionHandle` 内的 `connected=false` 首行短路
- `send_files_async` 内的批大小 / 单文件超限提示
- `TransferSlotLimiter` 并发限额行为（建议单独针对 `transfer_limit.rs` 加 unit test）

**Phase 3 验收**：
- [ ] `pnpm test:rust` 从 49 → 60 全绿
- [ ] `pnpm test:web` 保持 67 条全绿
- [ ] 11 个新用例都能在离线状态运行（无网络、无 Tauri）
- [ ] 11 个用例总耗时 < 5s

### Phase 4 — 双机手工冒烟验收（约 30 分钟）

**目标**：确认抽象没有引入行为回归；分派 4 分支在真实 WebRTC/直连上依然工作。

冒烟清单（两台 Windows 机器，同局域网）：

1. `pnpm build` 双机安装或 `pnpm dev` 双机启动。
2. 配对完成。
3. 复制文本 → 对端 5s 内出现在剪贴板 + 历史。
4. 复制小图（截图 < 512KB）→ 对端出现图片。
5. 复制大图（> 512KB, < 5MB）→ 进度条走完，对端出现。
6. 复制单文件（< 100MB）→ 进度条走完，对端出现文件。
7. 复制多文件（< 500MB 合计）→ 批传成功。
8. 关闭"自动同步剪贴板"开关，复制文本 → 对端不出现。
9. 关闭"同步图片"开关，复制小图 → 对端不出现。
10. 关闭"同步文件"开关，复制文件 → 对端出现 Text 快照（现状即如此）。

**Phase 4 验收**：
- [ ] 10 条冒烟全过
- [ ] 无 Rust panic / 前端 console error
- [ ] 结果与抽象前对齐（对照上一主题冒烟记录）

## 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|:---|:---|:---|:---|
| 4 分支等价性错位 | 低 | 高（生产回归） | Phase 3 单测锁 11 用例；Phase 4 双机冒烟兜底 |
| `Arc<ClipboardSnapshot>` 引入的克隆开销 | 低 | 低（<1%） | 快照本就在传输前克隆过一遍；`Arc::clone` 是引用计数 |
| `async-trait` 依赖增加 | 极低 | 极低 | 该 crate 稳定、编译期开销可忽略 |
| `RegistryOutProvider::active_outs` 每次事件都 `.lock().await` | 中 | 低 | 与现状 `active_handles()` 内的 `try_lock` 语义近似；仅短暂持锁 |
| `lib.rs` 构造点漏改导致 `SyncEngine::new` 签名错误 | 中 | 中（编译失败） | Phase 2 强制 `pnpm check` 才允许推进 |
| 冒烟发现分支回归 | 低 | 中 | 回滚 Phase 2 的 engine.rs / lib.rs 改动（out.rs 保留） |

## 验收标准

**必须满足**：

- [ ] Phase 1–4 全部完成
- [ ] `pnpm check` 通过
- [ ] `pnpm test`：前端 67 + Rust 57（+8）全绿
- [ ] `SyncEngine` 依赖清单不再包含 `AppHandle` / `ConnectionHandle` / `ConnectionRegistry`
- [ ] 双机冒烟 10 项全过
- [ ] `network/webrtc.rs` / `network/sessions.rs` / 前端零改动（`git diff --stat` 空）

**允许延后**：

- 更细粒度的失败路径断言（例如 `send_files_async` 内部限额提示）
- 用 `insta` 快照 `RecordedCall` 序列
- 把 `test_support` 提取到 `sync/out/test_support.rs` 独立文件

## 时间预估

| 阶段 | 预估 | 备注 |
|:---|:---|:---|
| Phase 1 | 60 min | trait + 生产包装 |
| Phase 2 | 90 min | engine.rs 重写 + lib.rs 构造点 |
| Phase 3 | 100 min | 11 个单测 |
| Phase 4 | 30 min | 双机冒烟 |
| **合计** | **约 4.7 小时** | 单人连续开发 |

## 用户确认记录（2026-07-11）

- [x] 抽象命名：`ClipboardOut` / `ClipboardOutProvider`
- [x] 新增依赖：`async-trait = "0.1"`
- [x] Phase 3 单测扩到 11 条：加 `sync_images_off` / `sync_files_off` / `empty_outs` 三条副条件；`connected=false` 短路与 `send_files_async` 限额留给后续 network 层主题

以上已确认。等 **"开始执行"** 后推进 Phase 1。