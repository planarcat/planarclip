# CLAUDE.md

本文件为在本仓库中协作的编码助手提供项目上下文、工作约定与安全边界。所有助手在执行任务前应优先遵守本文件；若与用户本轮明确要求冲突，应先向用户确认。

## 基本协作约定

- 默认使用简体中文回复；代码注释可使用英文或中文，优先使用英文。
- 先确认用户意图：用户只问原因、方案、怎么改时，只分析不直接改；用户明确说“直接改”“开始做”“按方案执行”时再落地修改。
- 修改代码前先阅读相关文件，避免基于猜测改动；写代码时保持与周围代码一致的命名、注释密度和风格。
- 删除、覆盖、重置、推送、发布、外部调用等难以回退或对外可见的操作，必须先确认。
- 如果本文件与 `AGENTS.md` 同时存在，应保持核心项目约定同步；`CLAUDE.md` 是 Claude Code 当前主要自动加载的项目说明，`AGENTS.md` 用于兼容其他 Agent 工具。

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

## UI 文案与交互约定

### 用户侧 UI 文案

- 用户可见的界面文本、按钮文案、状态文案、成功提示、警告提示、错误提示，默认全部使用中文。
- 提示文案必须使用自然语言，优先描述用户当前遇到的问题、系统正在做的事，以及用户下一步可以怎么做。
- 禁止直接向用户暴露程序术语、底层异常、协议名、库报错、系统调用错误、英文错误原文，除非用户明确进入开发排障场景。
- 如果底层返回的是技术错误，面向用户展示时必须先转换成自然语言；必要时可附带简短建议，例如检查网络、确认对端已启动、稍后重试。
- 成功提示应简洁明确，例如“已连接到设备”“配对成功”“已完成同步”；不要使用生硬的工程术语。
- 警告与失败提示应说明原因和影响，例如“未能连接到对方设备，请确认对方应用已打开”；不要只显示“连接失败”或原始错误对象。
- 同一类状态提示保持口径一致，避免同一流程里中英混用、术语混乱或一个地方说“配对”另一个地方说“握手”。
- 如果确实需要保留技术细节用于调试，应将技术细节写入日志或开发者输出，而不是直接展示在用户界面中。

### 交互与按钮规范

- 设计按钮等交互元素时，优先使用纯图标样式，尤其是刷新、增加、删除这类简单操作。
- 能用图标准确表达的按钮默认不再附带文字，但必须提供清晰的悬浮提示或无障碍标签。
- 连接、断开、刷新等高频动作应优先放在列表项右侧或标题操作区，保持就近操作与视觉统一。

## Figma MCP 约定

- 当前会话可使用 `figma-mcp-go` MCP 读取和操作 Figma 文件；使用前优先通过 `get_metadata` / `get_selection` / `get_design_context` 确认当前文件、页面与选区。
- 读取设计时优先使用 `get_design_context`，避免直接读取过大的完整文档树；需要精确节点信息时再用 `get_node` 或 `get_nodes_info`。
- 修改 Figma 前应确认目标节点、页面和操作意图；删除节点、删除页面、覆盖样式、批量重命名、导出文件等操作属于有副作用操作，需谨慎执行。
- Figma MCP 工具权限白名单应配置在 Claude Code 的 `settings.json` / `settings.local.json`，不要只写在本文档中；本文档只记录项目协作约定。

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **planarclip** (1250 symbols, 1955 relationships, 83 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
