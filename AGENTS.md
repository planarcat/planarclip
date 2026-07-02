# AGENTS.md

本文件是 PlanarClip 项目的**统一编码代理说明**，供 Cursor、Claude Code 等 AI 编码工具自动加载。原先独立的 `CLAUDE.md` 已合并至本文档；`CLAUDE.md` 仅保留为指向本文件的入口。更完整的产品与开发说明见 [`README.md`](./README.md)；前端 UI 与视觉规范见 [`Docs/UI_GUIDE.md`](./Docs/UI_GUIDE.md)。

## 基本协作约定

- 默认使用简体中文回复；代码注释可使用英文或中文，优先使用英文。
- 先确认用户意图：用户只问原因、方案、怎么改时，只分析不直接改；用户明确说“直接改”“开始做”“按方案执行”时再落地修改。
- 修改代码前先阅读相关文件，避免基于猜测改动；写代码时保持与周围代码一致的命名、注释密度和风格。
- 删除、覆盖、重置、推送、发布、外部调用等难以回退或对外可见的操作，必须先确认。

## 命令执行

- 禁止在同一次 shell 调用里编写多条命令；必须一条一条执行。

## 构建与运行

仓库根目录已配置 **pnpm workspace**，推荐在根目录执行以下命令（会转发到 workspace 包 `planarclip`，目录为 `Apps/`）：

```bash
pnpm install         # 在根目录安装 workspace 依赖（首次或锁文件变更后）
pnpm dev             # 启动完整应用（Tauri + Rust 后端 + 前端）
pnpm dev:web         # 仅启动前端开发服务器 (localhost:1420)
pnpm check           # 前端类型检查 + Rust cargo check
pnpm check:web       # 仅运行前端 TypeScript 类型检查
pnpm build           # 生成完整桌面应用安装包
pnpm build:web       # 仅构建前端产物
pnpm preview:web     # 预览前端构建结果
```

也可进入 `Apps/` 直接执行同名脚本（workspace 内仍可用）。

命名约定：`pnpm <动作>` 表示完整应用流程，`pnpm <动作>:web` 表示仅执行前端 Web 流程。前端开发服务器端口为 1420，HMR 端口为 1421。`tauri.conf.json` 通过 `beforeDevCommand` / `beforeBuildCommand` 调用 `pnpm dev:web` 与 `pnpm build:web`（在 `Apps/` 目录上下文中执行）。

## 工作区命令：手动 vs 载入自动

配置：`.vscode/tasks.json`（`runOn: folderOpen`）、`.vscode/settings.json`（`task.allowAutomaticTasks: on`）。需信任工作区；必要时 **Tasks: Manage Automatic Tasks** 允许自动任务。

### 手动模式（工作区根目录终端）

| 命令 | 行为 |
| --- | --- |
| `pnpm dev` | 启动完整 Tauri 应用 |
| `pnpm check:watch` | 前端 `tsc --watch` + Rust `cargo watch check` |
| `pnpm pull` | pull `origin/master`（有未提交改动会先 stash），成功后 `pnpm analyze` |
| `pnpm pull:only` | 只 pull，不 analyze |
| `pnpm analyze` | WSL 内 GitNexus `analyze --embeddings`（不 pull） |
| `pnpm analyze:planarclip` | 同上（单仓别名） |

`gitnexus:analyze` 为 `pnpm analyze` 别名。pull 分支见 `scripts/workspace-repos.mjs`。

### 自动模式（载入 PlanarClip 工作区）

每个命令 **独占一个终端**（`panel: dedicated`）：

| 终端 | 命令 |
| --- | --- |
| dev | `pnpm dev` |
| check:watch | `pnpm check:watch` |
| pull | `pnpm pull:open` → pull 成功 → **同终端** `pnpm analyze` |

手动复现自动 pull：**Tasks: Run Task** → **PlanarClip: pull（自动模式 · 分终端）**。

GitNexus 在 WSL 执行，`HF_ENDPOINT=https://hf-mirror.com`。无 WSL / gitnexus 时 pull 仍可用，analyze 会失败；可单独 `pnpm pull:only` 或在本机按 [execution-plan.md](./execution-plan.md) 配置 WSL 索引。

- 依赖异常时，在根目录执行 `pnpm install --config.confirmModulesPurge=false` 修复。

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
planarclip/                  # 仓库根目录（pnpm workspace）
├── package.json             # workspace 根：转发 dev/build/check 等脚本
├── pnpm-workspace.yaml      # workspace 成员与 allowBuilds 配置
├── pnpm-lock.yaml           # 锁文件（根目录统一维护）
├── Apps/                    # 桌面应用主体（package.json name: planarclip）
│   ├── package.json
│   ├── index.html
│   ├── src/
│   │   ├── main.tsx         # React 入口
│   │   ├── app/
│   │   │   ├── components/
│   │   │   │   ├── common/  # 通用小组件
│   │   │   │   ├── ui/      # UI 规范薄组件（ModalShell、PrimaryButton 等）
│   │   │   │   ├── layout/  # 侧栏与右侧概览面板
│   │   │   │   ├── overlays/# 配对弹层、入站连接确认
│   │   │   │   └── pages/   # 剪贴板 / 设备 / 设置页面
│   │   │   ├── constants/   # 主题常量
│   │   │   ├── hooks/       # 桌面桥接、配对流程、主题状态
│   │   │   ├── utils/       # 消息、设备、时间、设置等工具
│   │   │   ├── App.tsx      # 前端主装配
│   │   │   └── types.ts     # 前端类型定义
│   │   └── styles/          # 全局样式、主题样式
│   └── src-tauri/
│       ├── src/
│       │   ├── main.rs      # Rust 入口，windows_subsystem = "windows"
│       │   ├── lib.rs       # Tauri 应用组装：状态管理、Tauri 命令、系统托盘
│       │   ├── clipboard/   # 剪贴板监听与历史摘要
│       │   ├── crypto/      # 密钥生成与设备指纹
│       │   ├── network/     # 局域网发现、连接与传输
│       │   ├── sync/        # 剪贴板同步引擎与去重逻辑
│       │   ├── storage/     # 本地 JSON 配置加载/保存
│       │   └── tray/        # 系统托盘相关实现
│       ├── tauri.conf.json  # Tauri 配置：窗口 1280×820、默认隐藏、托盘图标
│       ├── tauri.dev.conf.json # 开发合并配置（静默启动 create: false 等）
│       └── capabilities/
```

## 当前状态

- 桌面端主流程已可用：文本 / 图片 / 文件剪贴板同步（Windows 双机验证）、局域网设备发现、6 位配对码与熟悉设备、托盘驻留、三栏 UI（剪贴板 / 设备 / 设置）。
- 配对弹层（`PairingModal`）支持按目标设备展示、可切换局域网设备列表、60 秒倒计时与配对码轮换（会话级动态码 + `rotate_pairing_code`）。
- 启动与窗口：支持开机自启与**静默启动**（仅托盘）；主窗 `create: false`，首次点托盘冷启动 WebView；关窗默认 **hide 到托盘**（保留 WebView，非预暖）；2026-07 **方案 E** 冷启动优化（`get_shell_bootstrap` / `get_shell_deferred`、`notify_main_ui_ready`、设置页懒加载）。
- 浏览器预览模式（`pnpm dev:web`）只能查看 UI；连接与配对能力需在 Tauri 桌面应用中体验。
- 默认局域网直连端口为 `19876`；前端开发服务器端口为 `1420`，HMR 端口为 `1421`。

## UI 与前端视觉

**完整规范**见 [`Docs/UI_GUIDE.md`](./Docs/UI_GUIDE.md)（用户文案、交互、设计令牌、组件 class 契约、动效、浮层 z-index、Figma 对照、样式入口）。

修改 `Apps/src/app/components` 或 `styles/` 时须遵循该文档。以下为代理须始终记住的摘要：

- 用户可见文案**默认中文**、自然语言；不暴露底层错误原文；成功/失败提示说明原因与下一步。
- 简单操作用**纯图标按钮**，须提供 `title` 或 `aria-label`；连接/断开/刷新放在列表右侧或标题区。
- 颜色与圆角用 **CSS 变量 / Tailwind 语义色**，禁止硬编码主题 hex；新卡片用 `rounded-xl`，模态用 `rounded-2xl`。
- 浮层 z-index：配对 `50` → 入站 `60` → 右下卡 `70` → `StatusNotice` `80`。
- 全局样式只改 **`Apps/src/styles/index.css`**；主题色定义在 `app/constants/theme.ts`。

## 跨设备隐私与协议

- 本机配置、开关状态、历史内容等默认不对他机暴露；他机仅能通过连接与同步协议中**对方主动下发的报文**获知有限信息。
- 接收端默认只回复完成传输所需的最小信息；需要额外字段时，应通过明确的协议回应提供，或在发送端请求且本机同意时再提供。
- 对外原因码（如 `TransferCancel`）禁止携带本机设置名、内部实现细节；使用约定的 `peer:*` 回应码，由发送端映射为用户可见文案。
- 发送端不得根据他机行为推断其配置并在 UI 中展示；若他机以等效方式处理了内容（如 `peer:handled`），发送端应视为成功。
- 连接发现等场景可暴露的必要信息限于设备名、局域网地址、端口等达成连接所需字段。

## Figma MCP

视觉与 Figma 对照流程见 [`Docs/UI_GUIDE.md` §10](./Docs/UI_GUIDE.md#10-figma-与设计对照)。使用 `figma-mcp-go` 时优先 `get_design_context`；有副作用的 Figma 操作须先确认；MCP 权限在 IDE 的 MCP 配置中维护。

## GitNexus — 代码智能

以下区块由 GitNexus 维护（`<!-- gitnexus:start -->` … `<!-- gitnexus:end -->`）。运行 `npx gitnexus analyze` 后会自动更新；请勿手工改写区块内正文。若工具仍向 `CLAUDE.md` 写入同类内容，请合并进本节并保持 `CLAUDE.md` 仅为指向本文档的入口。

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **planarclip** (3588 symbols, 6787 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
