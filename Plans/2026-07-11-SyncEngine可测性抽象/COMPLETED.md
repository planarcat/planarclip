# SyncEngine 可测性抽象 — 已执行确认

> 执行时间: 2026-07-11
> 基于方案: [execution-plan.md](execution-plan.md)

## 结论

**Phase 1–3 已 100% 落地**，Rust 单测从 49 → **60**（+11 分派单测），前端 67 保持全绿。`ConnectionHandle` / `webrtc.rs` / `sessions.rs` / 前端零改动。`SyncEngine` 本体已完全脱离 Tauri（无 `AppHandle` / `ConnectionHandle` / `ConnectionRegistry` 字段）。

Phase 4 双机手工冒烟为验收动作，需人工在两台机器上执行，见 UNEXECUTED.md。

## 已完成清单

### Phase 1 — trait 与生产包装 ✅

- `Apps/src-tauri/Cargo.toml`：`[dependencies]` 增 `async-trait = "0.1"`
- `Apps/src-tauri/src/sync/mod.rs`：新增 `pub mod out;`
- `Apps/src-tauri/src/sync/out.rs`：新建
  - `pub trait ClipboardOut`（4 个方法与 `ConnectionHandle` 现有 API 语义对齐）
  - `pub trait ClipboardOutProvider`（异步返回 `Vec<Arc<dyn ClipboardOut>>`）
  - `pub struct TauriConnectionOut { inner: ConnectionHandle, app: AppHandle }` + `impl ClipboardOut`
  - `pub struct RegistryOutProvider { registry, app }` + `impl ClipboardOutProvider`
- 验收：`pnpm check` 通过，`out.rs` 相关 6 条 warning 在 Phase 2 后消失

### Phase 2 — SyncEngine 与构造点替换 ✅

- `Apps/src-tauri/src/sync/engine.rs`：全部重写（133 → 129 行）
  - 字段：`app_handle` / `connections` 移除；`provider: Arc<dyn ClipboardOutProvider>` 新增
  - `broadcast_snapshot` 签名重构：接 `Vec<Arc<dyn ClipboardOut>>` + `Arc<ClipboardSnapshot>`
  - 4 分支等价性核对通过（chunked 大图 / 文件批 / 大图不支持分块回落 / 文本+内联+其他）
- `Apps/src-tauri/src/lib.rs`：唯一构造点改造
  - 新增 3 行 `Arc::new(RegistryOutProvider::new(...))`
  - `SyncEngine::new` 参数：`-app_handle_bg`、`+provider`
- 验收：
  - `pnpm check` 通过（新增依赖 `async-trait` 首次编译约 20s）
  - `sync::*` 7 条既有单测全绿
  - `SyncEngine` 字段完全脱 Tauri（`AppHandle` / `ConnectionHandle` / `ConnectionRegistry` 均不再出现在结构体上）

### Phase 3 — 测试实现与分派单测 ✅

- `Apps/src-tauri/src/sync/out.rs`：末尾追加 `#[cfg(test)] pub(crate) mod test_support`
  - `enum RecordedCall { Snapshot / ImageAsync / FilesAsync }`
  - `struct InMemoryOut { supports_chunked, calls }` + `impl ClipboardOut`
  - `struct StaticOutsProvider { outs }` + `impl ClipboardOutProvider`
- `Apps/src-tauri/src/sync/engine.rs`：末尾追加 `#[cfg(test)] mod tests`，**11 条用例全绿**：
  - A 分派 6 条：`text_snapshot_goes_to_all_outs_send_snapshot`、`small_image_uses_send_snapshot`、`large_image_with_chunked_uses_image_async`、`large_image_without_chunked_falls_back_to_send_snapshot`、`file_list_with_sync_uses_files_async`、`file_list_without_sync_falls_back_to_send_snapshot`
  - B 短路 2 条：`remote_origin_event_is_skipped`、`auto_sync_off_and_not_skip_history_is_skipped`
  - C 副条件 3 条：`sync_images_off_large_image_falls_back_to_send_snapshot`、`sync_files_off_file_list_falls_back_to_send_snapshot`、`empty_outs_list_does_not_panic_and_no_spawn`
- 验收：
  - `cargo test --lib sync::engine` → 11 passed，60ms 完成
  - `cargo test --lib` 全量 → 60 passed（49 + 11）
  - `pnpm test:web` → 67 passed
  - 全部离线运行，无网络、无 Tauri、无实际连接

## 影响面对照（vs 第 2 轮预判）

| 文件 | 预判 | 实际 | 一致 |
|:---|:---|:---|:---|
| `sync/engine.rs` | 全部重写 133 行 | 重写 129 行 + 追加 200 行测试 | ✅ |
| `sync/out.rs` | 新增 | 新建 250 行（含 test_support） | ✅ |
| `sync/mod.rs` | `pub mod out;` | +1 行 | ✅ |
| `lib.rs:3219` | 5 行以内 | +3 行 `RegistryOutProvider` 构造 | ✅ |
| `Cargo.toml` | 增 `async-trait` | +1 行 | ✅ |
| `network/webrtc.rs` | 零改动 | 零改动 | ✅ |
| `network/sessions.rs` | 零改动 | 零改动 | ✅ |
| 前端 `Apps/src/**` | 零改动 | 零改动 | ✅ |

## 数据指标

- **Rust 单测**：49 → 60（+22.4%）
- **前端单测**：67（不变）
- **测试耗时**：新增 11 条 60ms，全量 `cargo test --lib` 完成时间 <1s
- **编译产物影响**：`async-trait` 首次编译约 5s；后续增量编译无感
- **代码行数净变**：`engine.rs` 净减 4；`out.rs` 净增 250；`lib.rs` 净增 3

## 已知遗留（非本主题引入）

- `storage::json::tests::save_then_load_roundtrip_preserves_fields`：上一主题遗留 flaky（`APPDATA` 环境变量的 EnvGuard drop 顺序在多线程下有 race）。本次全量执行时 0 次触发；上次执行时 1 次触发。已记入《测试基建搭建》主题的 UNEXECUTED，与本次抽象无关。