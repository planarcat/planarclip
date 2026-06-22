# CLAUDE.md

本文件为在本仓库中协作的编码助手提供上下文与工作约定。

## 构建与运行

所有命令在 `Apps/planarclip/` 目录下执行：

```bash
pnpm install         # 安装依赖
pnpm dev             # 启动完整应用（Tauri + Rust 后端 + 前端）
pnpm dev-web         # 仅启动前端开发服务器 (localhost:1420)
pnpm check           # 前端检查 + Rust cargo check
pnpm check-web       # 仅运行前端格式、lint、类型检查
pnpm build           # 生成完整桌面应用安装包
pnpm build-web       # 仅构建前端产物
pnpm preview-web     # 预览前端构建结果
```

命名约定：`pnpm <动作>` 表示完整应用流程，`pnpm <动作>-web` 表示仅执行前端 Web 流程。前端开发服务器端口为 1420，HMR 端口为 1421。`tauri.conf.json` 仍通过 `beforeDevCommand` / `beforeBuildCommand` 调用 `pnpm exec vp dev` 与 `pnpm exec vp build`，避免 Tauri 子进程无法解析本地 Vite+ CLI。

## 技术栈

- **桌面框架**: Tauri v2（Rust 后端 + WebView 前端）
- **前端**: Vite+ + TypeScript，单页面应用，内联 CSS
- **剪贴板访问**: `arboard`
- **异步运行时**: `tokio`
- **发现与连接**: WebSocket 信令、mDNS、局域网 TCP 直连
- **加密基础**: `x25519-dalek` + `blake3`
- **持久化**: 本地 JSON 配置文件（Windows 上存于 `%APPDATA%/planarclip_config.json`）

## 项目结构

```text
Apps/planarclip/
├── src/
│   └── main.ts              # 前端入口，UI 逻辑（连接状态、配对输入、局域网设备列表）
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
│   │   │   └── keys.rs      # KeyPair 生成与设备指纹
│   │   ├── network/
│   │   │   ├── mod.rs
│   │   │   ├── signalling.rs # WebSocket 信令客户端连接
│   │   │   ├── discovery.rs  # mDNS 设备发现
│   │   │   ├── direct.rs     # 局域网直连握手与数据通道
│   │   │   └── webrtc.rs     # 连接管理与剪贴板消息收发
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
│   ├── tauri.conf.json       # Tauri 配置：窗口 380x520、默认隐藏、托盘图标
│   └── capabilities/
└── index.html                # 前端入口 HTML，内联深色主题 CSS
```

## 当前状态

- 前端命令已统一切换到 Vite+ (`vp`)，并通过 `pnpm <动作>` / `pnpm <动作>-web` 做了统一封装。
- Tauri 的 `beforeDevCommand` / `beforeBuildCommand` 仍使用 `pnpm exec vp dev` / `pnpm exec vp build`，确保子进程能解析本地 Vite+ CLI。
- `cargo build`、`pnpm dev`、`pnpm build` 在用户本机均已验证通过。

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **planarclip** (1149 symbols, 1736 relationships, 66 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
