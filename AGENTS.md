# AGENTS.md

本文件是 PlanarClip 项目的编码代理说明，供 Cursor、Claude Code 等支持 `AGENTS.md` 的工具自动加载。更完整的产品与开发说明见仓库根目录 `README.md`。

## 基本协作约定

- 默认使用简体中文回复；代码注释可使用英文或中文，优先使用英文。
- 先确认用户意图：用户只问原因、方案、怎么改时，只分析不直接改；用户明确说“直接改”“开始做”“按方案执行”时再落地修改。
- 修改代码前先阅读相关文件，避免基于猜测改动；写代码时保持与周围代码一致的命名、注释密度和风格。
- 删除、覆盖、重置、推送、发布、外部调用等难以回退或对外可见的操作，必须先确认。

## 命令执行

- 禁止在同一次 shell 调用里编写多条命令；必须一条一条执行。

## 构建与运行

所有命令在 `Apps/planarclip/` 目录下执行：

```bash
pnpm install         # 安装依赖
pnpm dev             # 启动完整应用（Tauri + Rust 后端 + 前端）
pnpm dev:web         # 仅启动前端开发服务器 (localhost:1420)
pnpm check           # 前端类型检查 + Rust cargo check
pnpm check:web       # 仅运行前端 TypeScript 类型检查
pnpm build           # 生成完整桌面应用安装包
pnpm build:web       # 仅构建前端产物
pnpm preview:web     # 预览前端构建结果
```

命名约定：`pnpm <动作>` 表示完整应用流程，`pnpm <动作>:web` 表示仅执行前端 Web 流程。前端开发服务器端口为 1420，HMR 端口为 1421。`tauri.conf.json` 通过 `beforeDevCommand` / `beforeBuildCommand` 调用 `pnpm dev:web` 与 `pnpm build:web`。

## 技术栈

- **桌面框架**: Tauri 2（Rust 后端 + WebView 前端）
- **前端**: React 18 + TypeScript + Vite 8 + Tailwind CSS 4
- **UI 图标**: lucide-react
- **剪贴板访问**: arboard
- **异步运行时**: tokio
- **发现与连接**: mDNS、局域网 TCP 直连
- **加密基础**: x25519-dalek + blake3
- **持久化**: 本地 JSON 配置文件（Windows 上存于 `%APPDATA%/planarclip_config.json`）

## 项目结构

```text
Apps/planarclip/
├── src/
│   ├── app/
│   │   ├── components/
│   │   │   ├── common/      # 通用小组件
│   │   │   ├── layout/      # 侧栏与右侧概览面板
│   │   │   ├── overlays/    # 配对弹层、入站连接确认
│   │   │   └── pages/       # 剪贴板 / 设备 / 设置页面
│   │   ├── constants/       # 主题常量
│   │   ├── hooks/           # 桌面桥接、配对流程、主题状态
│   │   ├── utils/           # 消息、设备、时间、设置等工具
│   │   ├── App.tsx          # 前端主装配
│   │   └── types.ts         # 前端类型定义
│   ├── styles/              # 全局样式、主题样式
│   └── main.tsx             # React 入口
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Rust 入口，windows_subsystem = "windows"
│   │   ├── lib.rs           # Tauri 应用组装：状态管理、Tauri 命令、系统托盘
│   │   ├── clipboard/       # 剪贴板监听与历史摘要
│   │   ├── crypto/          # 密钥生成与设备指纹
│   │   ├── network/         # 局域网发现、连接与传输
│   │   ├── sync/            # 剪贴板同步引擎与去重逻辑
│   │   ├── storage/         # 本地 JSON 配置加载/保存
│   │   └── tray/            # 系统托盘相关实现
│   ├── tauri.conf.json      # Tauri 配置：窗口 1280×820、默认隐藏、托盘图标
│   └── capabilities/
└── index.html               # 前端入口 HTML
```

## 当前状态

- 桌面端主流程已可用：文本剪贴板同步、局域网设备发现、6 位配对码确认、可信设备持久化、托盘驻留、三栏 UI（剪贴板 / 设备 / 设置）。
- 浏览器预览模式（`pnpm dev:web`）只能查看 UI；连接与配对能力需在 Tauri 桌面应用中体验。
- 当前仅支持文本同步；图片与文件同步尚未接入。
- 默认局域网直连端口为 `19876`。

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
- Figma MCP 工具权限白名单应配置在 IDE 的 MCP 设置中（如 `.cursor/mcp.json`、Claude Code 的 `settings.json` / `settings.local.json`），不要只写在本文档中；本文档只记录项目协作约定。

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **planarclip** (1293 symbols, 2072 relationships, 95 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
