# 第 2 轮：销毁 WebView 后剪贴板能力 + 连接请求系统通知

> 时间: 2026-06-26

## 用户提问（原文）

将关窗销webview的讨论放到主题中

然后回答：销毁webview后，还能正常使用剪贴板吗？包括监听、收到他机发送的内容后放到本机剪贴板等，如果是收到连接请求，调用系统通知打开窗口进行连接相关操作

## 背景与分析

### 架构分层：Rust 后端 vs WebView 前端

PlanarClip 的核心同步能力在 **Rust 侧独立线程** 运行，与 WebView 生命周期解耦：

```
App 启动 (setup)
├── mDNS 发现          → Rust async task
├── TCP 监听           → Rust async task
├── ClipboardMonitor   → 独立 std::thread + tokio（150ms 轮询 arboard）
├── SyncEngine         → 同上线程（broadcast 收本地剪贴板 → 发往对端）
├── connect_direct     → 收远端 Clipboard 消息 → ClipboardMonitor::write_clipboard
└── WebView (React)    → 仅 UI：历史展示、配对弹层、设置
```

关键代码路径（均不经过 WebView）：

| 能力 | 实现位置 | 依赖 WebView？ |
|------|----------|----------------|
| 监听本机剪贴板 | `clipboard/monitor.rs` → `clip_tx.send` | ❌ |
| 发送本机剪贴板到对端 | `sync/engine.rs` → `ConnectionHandle::send_clipboard` | ❌ |
| 收到对端内容写入本机 | `network/webrtc.rs` → `ClipboardMonitor::write_clipboard` | ❌ |
| 剪贴板历史持久化 | `lib.rs` → `persist_clipboard_history` | ❌ |
| 剪贴板历史 UI 刷新 | `emit("clipboard-history-changed")` → React | ✅ 仅 UI |
| 入站连接（自动接受） | `handle_incoming_connection` → `should_auto_accept` | ❌ |
| 入站连接（需用户确认） | `emit("connection-request")` + 前端弹层 | ✅ 当前实现 |
| 用户点「接受/拒绝」 | 前端 `accept_connection` / `reject_connection` invoke | ✅ 当前实现 |

### 销毁 WebView 后：剪贴板 — **可以正常使用**

**结论：销毁 WebView 不影响剪贴板同步的核心能力。**

1. **监听本机剪贴板**：`ClipboardMonitor` 在独立线程持续轮询，与窗口无关。
2. **发送到已连接设备**：`SyncEngine` 通过 `ConnectionHandle` 经 TCP 直连发送，与窗口无关。
3. **收到远端内容写入本机**：`connect_direct` 收到 `SignalMessage::Clipboard` 后直接调用 `ClipboardMonitor::write_clipboard(&payload)`（arboard），**不经过 WebView**。
4. **历史记录**：Rust 侧仍会 `merge_clipboard_history` + `persist_clipboard_history` 写 JSON；只是 `emit` 到前端时 WebView 不存在会被忽略。下次打开窗口时 `get_clipboard_history` 可恢复 UI。

**用户无感知差异**：复制/粘贴同步照常；仅「剪贴板历史页面不实时刷新」——关窗期间新条目落盘但不展示，开窗后可见。

### 销毁 WebView 后：连接请求 — **需改造**

| 场景 | 销毁 WebView 后 | 方案 |
|------|-----------------|------|
| 已信任设备 + 自动接受 | ✅ 完全正常，Rust 直接握手 | 无需 UI |
| 熟悉设备 + 需用户确认 | ❌ 当前依赖 `show_main_window` + React 弹层 | **系统通知 → 点击重建窗口 → 现有 `IncomingConnectionPrompt`** |
| 陌生设备 + 配对码 | ❌ 同上 | 同上 + 窗口内配对流程 |

当前代码在需确认时已调用 `show_main_window`（`lib.rs` ~1018 行），但 `hide` 模式下 WebView 仍在；改为 `destroy` 后 `show_main_window` 须改为 **异步 recreate**，且若用户未看到窗口，需 **Windows Toast 通知**：

> 「{设备名} 请求连接，点击打开 PlanarClip 确认」

点击通知 → 重建 WebView → 前端 `useConnectionBridge` 挂载 → 收到 pending 的 `connection-request`（或 Rust 侧重发一次）→ 显示 `IncomingConnectionPrompt`。

**注意**：Rust 侧 `pending_accept_tx` / `pending_reject_tx` 已在等待，窗口晚开几秒仍可接受/拒绝，但有过期/超时策略需在方案中定义。

### 系统通知技术选型

项目当前 **未集成** 通知插件。建议：

- `tauri-plugin-notification`（Tauri 2 官方插件，Windows Toast）
- 在 `handle_incoming_connection` 需确认分支：若 `get_webview_window("main")` 不存在或不可见 → 发通知 + 异步 `recreate_main_window`
- 通知点击回调：与托盘「打开 PlanarClip」共用 `ensure_main_window()` 逻辑

## 建议与回答

### 剪贴板

**可以。** 销毁 WebView 后，监听、发送、接收写入本机剪贴板均由 Rust 后端完成，与 UI 无关。这是采用 Tauri「重后端、轻前端」架构的主要优势。

### 连接请求

**不能沿用现状，需按用户意图改造：**

- 自动接受：无需改动
- 需确认：**系统通知 + 点击打开/重建窗口 + 现有连接确认 UI**
- 不在通知里直接「接受/拒绝」（首版），避免与配对码流程、安全确认逻辑分叉

### 内存预期（泄漏修复 + 关窗 destroy 后）

| 状态 | 预期内存 |
|------|----------|
| 窗口打开 | ~300 MB（WebView2 基线） |
| 关窗 destroy 后 | ~10–20 MB（Rust + 托盘） |
| 长期后台（泄漏已修） | 稳定在 ~10–20 MB，不再爬升 |

## 本轮要点

- 剪贴板同步：**不依赖 WebView**，destroy 后核心功能正常
- 剪贴板历史 UI：关窗期间不刷新，开窗后拉取即可
- 连接请求：自动接受无需 UI；需确认时走 **系统通知 → 重建窗口 → 连接弹层**
- 需新增 `tauri-plugin-notification` 与 `ensure_main_window` / `destroy_main_window` 窗口生命周期模块
