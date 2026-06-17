# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

PlanarClip 是一个基于 Tauri v2 的跨设备剪贴板同步工具。通过 WebRTC 点对点连接，在两台设备之间实时同步剪贴板文本内容，配合信令服务器进行设备发现和配对。

## 构建与运行

所有命令在 `Apps/planarclip/` 目录下执行：

```bash
npm run dev          # 启动 Vite 开发服务器 (localhost:1420)
npm run build        # TypeScript 编译 + Vite 构建
npx tauri dev        # 启动 Tauri 开发模式（含 Rust 后端）
npx tauri build      # 生产构建
```

前端开发服务器端口为 1420，HMR 端口为 1421。

## 技术栈

- **桌面框架**: Tauri v2（Rust 后端 + WebView 前端）
- **前端**: Vite v6 + TypeScript，单页面应用，内联 CSS
- **剪贴板访问**: `arboard`（Rust 跨平台剪贴板库）
- **加密**: X25519 密钥交换 (`x25519-dalek`)，BLAKE3 内容哈希
- **网络**: `tokio-tungstenite`（WebSocket 信令），WebRTC（P2P 数据传输，待实现）
- **异步运行时**: Tokio (`features = ["full"]`)
- **存储**: 本地 JSON 配置文件（Windows 上存于 `%APPDATA%/planarclip_config.json`）

## 项目结构

```
Apps/planarclip/
├── src/
│   └── main.ts              # 前端入口，UI 逻辑（连接状态、配对输入）
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Rust 入口，windows_subsystem = "windows"
│   │   ├── lib.rs           # Tauri 应用组装：状态管理、Tauri 命令、系统托盘
│   │   ├── clipboard/
│   │   │   ├── mod.rs
│   │   │   ├── monitor.rs   # ClipboardMonitor：每 500ms 轮询剪贴板，BLAKE3 哈希去重
│   │   │   └── types.rs     # ClipboardSnapshot 枚举（Text/Empty）+ 内容哈希
│   │   ├── crypto/
│   │   │   ├── mod.rs
│   │   │   └── keys.rs      # KeyPair 生成、X25519 DH 共享密钥、6 位配对码
│   │   ├── network/
│   │   │   ├── mod.rs
│   │   │   ├── signalling.rs # WebSocket 信令客户端连接
│   │   │   └── webrtc.rs     # WebRTC 连接管理（MVP 阶段为 stub）
│   │   ├── sync/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs     # SyncEngine：广播通道接收剪贴板变更，转发给对端
│   │   │   └── dedup.rs      # DedupStore：环形去重集合，防止同步循环
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   └── json.rs       # AppConfig 加载/保存，含设备名、密钥对、对端信息
│   │   ├── tray/
│   │   │   ├── mod.rs
│   │   │   └── menu.rs       # 系统托盘菜单（stub，实际在 lib.rs 中构建）
│   │   └── util/
│   │       ├── mod.rs
│   │       └── hash.rs       # BLAKE3 哈希辅助函数
│   ├── tauri.conf.json       # Tauri 配置：窗口 380x520、默认隐藏、托盘图标
│   └── capabilities/
└── index.html               # 前端入口 HTML，内联深色主题 CSS
```

## 关键架构决策

**数据流**: 剪贴板由 ClipboardMonitor 在后台每隔 500ms 轮询，变更通过 `tokio::sync::broadcast` 通道发送。SyncEngine 订阅该通道，在已连接状态下将变更推送给 WebRTC 对端。

**自写检测**: ClipboardMonitor 使用全局 `AtomicBool` 标志 `SELF_WRITING`，在程序自身写入剪贴板时跳过变更检测，防止同步循环。

**配对流程**: MVP 阶段使用 6 位数字配对码（从 X25519 公钥指纹提取），蓝牙配对风格的 UX。目前 `pair` 命令接受任意 6 位码并标记为已连接——完整的加密方案待实现。

**窗口管理**: 主窗口默认隐藏，通过系统托盘左键点击切换显示/隐藏。托盘菜单提供 "Show PlanarClip" 和 "Quit" 操作。

## 当前开发状态

- 剪贴板监控和变更检测：已完成
- 加密密钥对生成和配对码：已完成
- 系统托盘和窗口管理：已完成
- 前端基础 UI（状态、配对界面）：已完成
- WebSocket 信令客户端：已完成（连接逻辑）
- WebRTC P2P 数据传输：待实现
- 完整加密配对和双向同步：待实现

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **planarclip** (541 symbols, 812 relationships, 31 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/planarclip/context` | Codebase overview, check index freshness |
| `gitnexus://repo/planarclip/clusters` | All functional areas |
| `gitnexus://repo/planarclip/processes` | All execution flows |
| `gitnexus://repo/planarclip/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
