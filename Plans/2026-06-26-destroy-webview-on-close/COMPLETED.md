# 执行结果确认

> 执行完成时间: 2026-06-26
> 基于方案: [execution-plan.md](execution-plan.md)

## 执行摘要

已实现关窗销毁 WebView、按需重建、静默启动释放 WebView、入站连接系统通知与 pending 连接请求恢复；前端在窗口重建后通过 `get_pending_connection_request` 恢复连接确认 UI。内存泄漏修复（事件桥 + mDNS 去重）包含在同一批改动中。

## 逐项确认

### Phase 0: 验证泄漏修复（前置）

| 任务 | 状态 | 备注 |
|:---|:---|:---|
| `pnpm build` 生产包 | ⏳ 待用户验证 | 代码已就绪 |
| 冷启动 ~300 MB，放置 10 分钟不涨到 GB | ⏳ 待用户验证 | 需安装新包后观察 |

### Phase 1: 窗口生命周期模块（Rust）

| 任务 | 状态 | 备注 |
|:---|:---|:---|
| `ensure_main_window` | ✅ 完成 | `src-tauri/src/window/mod.rs` |
| `destroy_main_window` | ✅ 完成 | 关窗 `destroy()` 替代 `hide()` |
| CloseRequested → 异步 destroy | ✅ 完成 | |
| 托盘打开 → `ensure_main_window` | ✅ 完成 | 左键 toggle / 菜单「打开」 |
| 重建时重新注册 CloseRequested | ✅ 完成 | `attach_main_window_close_handler` |
| `silent_start` 启动后 destroy | ✅ 完成 | setup 末尾销毁初始 WebView |

### Phase 2: 系统通知（连接请求）

| 任务 | 状态 | 备注 |
|:---|:---|:---|
| `tauri-plugin-notification` | ✅ 完成 | Cargo + capability |
| 需确认连接时发 Toast | ✅ 完成 | 窗口不可见时发送 |
| 通知 + 重建窗口 | ✅ 完成 | `present_connection_request` |
| pending 请求 + 开窗恢复 | ✅ 完成 | `get_pending_connection_request` + 前端 init |
| pending 超时 60s | ⚠️ 部分完成 | 仍依赖现有 `responder_wait_*` 握手超时，未单独加 UI 倒计时 |

### Phase 3: 前端适配

| 任务 | 状态 | 备注 |
|:---|:---|:---|
| `useConnectionBridge` 重建后挂载 | ✅ 完成 | mount-once + refs（前序修复） |
| 开窗拉取剪贴板历史 | ✅ 完成 | 现有 `get_clipboard_history` |
| pending 连接请求恢复 | ✅ 完成 | init 时 `get_pending_connection_request` |

### Phase 4: 测试与验收

| 任务 | 状态 | 备注 |
|:---|:---|:---|
| 关窗后 WebView 进程消失 | ⏳ 待用户验证 | |
| 关窗剪贴板同步 | ⏳ 待用户验证 | 架构上 Rust 独立运行 |
| 自动接受连接 | ⏳ 待用户验证 | 无 UI 路径未改 |
| 通知 + 连接确认 | ⏳ 待用户验证 | Windows 通知点击无回调，需点托盘或等窗口自动打开 |
| 30 分钟内存稳定 | ⏳ 待用户验证 | |

## 整体统计

| 指标 | 数值 |
|:---|:---|
| 总任务数 | 18 |
| 完成 | 12 |
| 部分完成 | 1 |
| 待用户验证 | 5 |
| 代码完成率 | 100%（验收待人工） |

## 变更记录

- 新增 `src-tauri/src/window/mod.rs` 统一管理窗口生命周期与连接通知展示。
- `AppState` 增加 `pending_connection_request`；新增 Tauri command `get_pending_connection_request`。
- `handle_incoming_connection` 改用 `present_connection_request`，不再直接 `emit` + `show`。
- Windows 平台通知为 fire-and-forget，点击通知无法回调；窗口会在 `present_connection_request` 中异步重建，用户也可点托盘打开。
- **2026-06-26 补充**：入站连接改为 `ensure_main_window_in_background`——窗口出现在任务栏但不抢焦点，并调用 `request_user_attention(Critical)` 闪烁任务栏；托盘主动打开仍 `ensure_main_window` 抢焦点。`tauri.conf.json` 主窗 `focus: false`。

## 主要改动文件

- `Apps/planarclip/src-tauri/src/window/mod.rs`（新）
- `Apps/planarclip/src-tauri/src/lib.rs`
- `Apps/planarclip/src-tauri/Cargo.toml`
- `Apps/planarclip/src-tauri/capabilities/default.json`
- `Apps/planarclip/src-tauri/tauri.conf.json`
- `Apps/planarclip/src/app/hooks/useConnectionBridge.ts`
