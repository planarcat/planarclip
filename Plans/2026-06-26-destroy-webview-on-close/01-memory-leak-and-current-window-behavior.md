# 第 1 轮：WebView 内存问题与关窗 hide 现状

> 时间: 2026-06-26

## 用户提问（原文）

（综合前置对话）

- webview占用了巨大的内存，为什么？
- 这个内存是生产环境下的，不是开发环境的，我都没启动开发环境
- 退出重启后，不过300m相对于我对这款应用的预期来说也挺大的
- 放置几分钟后内存暴涨
- 可以关窗后释放webview吗

## 背景与分析

### 现象

| 状态 | WebView2: PlanarClip 内存 | 说明 |
|------|----------------------------|------|
| 冷启动（生产） | ~180–335 MB | WebView2 多进程合计，属 Chromium 基线 |
| 放置数分钟 | ~2.4 GB | 异常泄漏，非正常基线 |
| Rust 主进程 | ~10–20 MB | 与 WebView 分离 |

生产环境配置文件 `%APPDATA%\planarclip_config.json` 约 1.5 KB，无巨型剪贴板历史落盘。

### 泄漏根因（已在代码中修复，待打包验证）

1. mDNS `ServiceResolved` 频繁触发 → 后端每次 `emit("lan-devices-changed")`（即使列表未变）
2. 前端 `setLanDevices` → App 重渲染
3. `onPairingCodeRotated` 每次渲染为新函数 → `useConnectionBridge` 的 `useEffect` 整段重跑
4. 每次重跑：卸载 7 个 Tauri 监听 → 再注册 7 个 → 再 `invoke` 7 次
5. 托盘常驻 + WebView 不销毁 → 数分钟内 IPC/监听器堆积至 GB 级

**已实施修复（代码层，待用户 `pnpm build` 验证）：**

- `useConnectionBridge`：回调走 `useRef`，effect 仅在 `[tauriAvailable]` 时挂载一次
- 后端：设备列表仅在实际变化时 `emit`
- 前端：`areLanDevicesEqual` 去重后再 `setState`
- `App.tsx`：`handlePairingCodeRotated` 使用 `useCallback`

### 当前关窗行为

```rust
// lib.rs — 关窗仅 hide，WebView/Chromium 仍存活
fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}
```

关窗后仍占用 ~200–300 MB WebView 内存；Rust 同步逻辑继续运行。

### 关窗释放 WebView 的可行性（Tauri 2）

- **可以**：关窗时用 `win.destroy()` 销毁窗口与 WebView；托盘打开时用 `WebviewWindowBuilder::from_config(...).build()` 异步重建
- **Windows 约束**：创建窗口须在 `async` command 或 `tauri::async_runtime::spawn` 中执行，不可在 `on_window_event` 同步路径里直接 `build()`（WebView2 死锁风险）
- **预期收益**：关窗后 WebView 内存基本释放，进程仅剩 Rust 后端 ~10–20 MB

## 建议与回答

生产环境 ~300 MB 冷启动是 **Tauri + WebView2（Chromium）架构下限**，不是业务数据撑大；数分钟涨至 GB 是 **泄漏 bug**，与开发模式无关。

关窗释放 WebView **技术上可行且对产品形态合理**（托盘常驻、多数时间在后台同步），但需单独设计：

- 入站连接确认 UI 在 WebView 不存在时的替代路径
- 窗口重建与事件重新订阅
- 剪贴板同步是否依赖 WebView（见第 2 轮）

## 本轮要点

- 300 MB 冷启动：WebView2 基线，可接受但相对「小工具」偏大
- GB 级暴涨：mDNS + 事件桥反复重建导致泄漏，已修代码待验证
- 当前关窗 = `hide()`，不释放 WebView
- 关窗 `destroy()` + 按需重建：可行，需配套连接请求与通知方案
