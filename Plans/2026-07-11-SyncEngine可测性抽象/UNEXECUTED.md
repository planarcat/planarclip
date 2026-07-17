# SyncEngine 可测性抽象 — 未执行/待用户执行

> 生成时间: 2026-07-11

## 需用户手动执行

### Phase 4 — 双机手工冒烟验收（本次未执行）

**为什么没执行**：只有单台开发机；双机冒烟需要局域网内两台 Windows 机器同时安装/启动 PlanarClip。

**用户执行清单**（在两台机器上，同一局域网）：

1. 两机各自执行 `pnpm build` 生成 MSI 安装（或双机同时 `pnpm dev`）
2. 完成一次配对
3. 复制文本 → 对端 5s 内出现在剪贴板 + 历史
4. 复制小图（截图 < 512KB）→ 对端出现图片
5. 复制大图（512KB ~ 5MB PNG）→ 进度条走完，对端出现
6. 复制单个 < 100MB 文件 → 进度条走完，对端出现文件
7. 复制多文件（合计 < 500MB）→ 批传成功
8. 关闭"自动同步剪贴板" → 复制文本，对端不出现
9. 关闭"同步图片" → 复制小图，对端不出现
10. 关闭"同步文件" → 复制文件，对端不出现（现状会走 Text 快照，但因为 sync 关闭，接收端不该产生对应记录）

**预期**：10 条全过、无 Rust panic、无前端 console error、结果与抽象前一致。

## 已知遗留（非本主题范围，供后续主题参考）

### 1. `storage::json::tests::save_then_load_roundtrip_preserves_fields` flaky

- **触发**：多线程下 `EnvGuard::Drop` 在其他并发用例仍持有 `APPDATA` 覆盖期时回滚，导致 `load_config` 读到真机路径
- **规避**：`RUST_TEST_THREADS=1` 或用例内自建"独立 config_path"
- **来源**：上一主题《测试基建搭建》
- **本主题状态**：非本次改动引入；本次多次全量执行仅 1 次触发

### 2. `ConnectionHandle` 内部失败路径未覆盖

- 属 network 层职责，本主题范围外：
  - `send_snapshot` / `send_*_async` 内的 `connected=false` 首行短路
  - `send_files_async` 内的批大小 / 单文件超限提示
  - `send_image_async` 内的 512KB / 5MB 阈值提示
- **建议**：后续单独立题《webrtc 传输失败路径可测化》，把 `ConnectionHandle` 也做一次可测性抽象

### 3. `TransferSlotLimiter` 并发限额行为

- 目前只作为 fixture 传入 SyncEngine 单测，未直接断言限额行为
- **建议**：给 `transfer_limit.rs` 单独加 3~4 条 unit test（`acquire` 阻塞、`Drop` 释放、并发 N > limit 时的序列化）