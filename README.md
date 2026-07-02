# PlanarClip

PlanarClip（安装包产品名 **二向贴**）是一个基于 **Tauri 2 + React 18 + TypeScript + Rust** 构建的跨设备剪贴板同步桌面应用。

当前版本已经完成桌面端主流程：

- 文本 / 图片 / 文件剪贴板监听与同步（Windows 已双机验证；macOS 文件链路未全面接入）
- 局域网设备自动发现与直连
- 6 位配对码确认、会话级动态配对码与 60 秒倒计时轮换、熟悉设备持久化
- 配对弹层支持按目标设备展示、可切换设备列表与入站连接确认
- 托盘驻留、静默启动（仅托盘）、关窗收起到托盘（保留 WebView，再次打开更快）
- 2026-07 冷启动首屏优化：分级 shell IPC、设置页懒加载、主窗 UI 就绪后再展示
- 外观与行为设置持久化（明暗模式 / 主题色、启动项、同步开关、系统通知等）

## 当前能力

### 已实现

- 剪贴板文本 / 图片 / 文件自动同步（Windows；2026-06-29 双机联调：大图分块、批次、资源管理器粘贴）
- 局域网设备发现与连接、熟悉设备 / 信任与自动接受入站
- 配对码验证、会话级动态码轮换与连接请求确认；入站连接系统通知（可关闭）
- 剪贴板历史摘要（默认 100 条，可选 25 / 50 / 100 / 200 / 500；列表或网格视图）
- 设备页、剪贴板页、设置页三栏桌面 UI
- 系统托盘、开机自启、静默启动、关窗收起到托盘或直接退出
- 同步相关设置（图片 / 文件开关、大小上限、接收文件保存目录等）

### 当前限制

- macOS 上图片 / 文件同步未与 Windows 同级验证
- 浏览器预览模式（`pnpm dev:web`）只能查看 UI，连接与配对需在 Tauri 桌面应用中体验
- 默认局域网直连端口为 `19876`；后端当前仍维持单条活跃连接会话
- 静默启动下首次点托盘仍需冷启动 WebView（已做首屏优化，无后台预暖主窗）

## 技术栈

- **桌面框架**：Tauri 2
- **前端**：React 18 + TypeScript + Vite 8 + Tailwind CSS 4
- **后端**：Rust + Tokio
- **局域网发现**：mDNS
- **传输链路**：局域网 TCP 直连
- **剪贴板**：arboard
- **加密基础**：x25519-dalek + blake3
- **图标 / UI 组件**：lucide-react

## 开发环境

建议先准备以下环境：

- Node.js 20+
- pnpm 11+
- Rust stable
- Tauri 2 对应平台依赖

项目默认使用 **pnpm** 作为包管理器。仓库根目录为 **pnpm workspace**，应用主体位于 `Apps/`（workspace 包名 `planarclip`）。推荐在根目录执行 `pnpm install` 与 `pnpm dev` 等命令。前端开发服务器默认端口为 `1420`，HMR 端口为 `1421`。

工作区载入自动任务与 pull / GitNexus 脚本说明见 [AGENTS.md](./AGENTS.md#工作区命令手动-vs-载入自动)。

## 安装与运行

在仓库**根目录**执行：

```bash
pnpm install
```

首次切换到 workspace 布局后，若根目录尚无 `pnpm-lock.yaml`，上述命令会生成根级锁文件。也可进入 `Apps/` 执行同名脚本，但安装依赖仍建议在根目录完成。

### 开发模式

```bash
# 启动完整桌面应用（前端 + Tauri + Rust）
pnpm dev

# 仅启动前端开发服务器
pnpm dev:web

# 完整检查（前端类型检查 + Rust cargo check）
pnpm check

# 仅执行前端类型检查
pnpm check:web
```

### 构建

```bash
# 构建桌面安装包
pnpm build

# 仅构建前端静态产物
pnpm build:web

# 本地预览前端构建结果
pnpm preview:web
```

### 常用底层命令

```bash
pnpm tauri dev
pnpm tauri build
```

## 使用方式

1. 启动桌面应用后，配对弹层或设备页会展示当前设备的 6 位配对码；入站配对时会话码约 60 秒后自动轮换。
2. 在“设备”页可查看已发现的局域网设备，点击连接后打开配对弹层，并可在弹层内切换目标设备。
3. 若对方主动请求连接，应用会弹出配对确认弹层，请让对方输入你设备上显示的配对码。
4. 建立连接后，文本剪贴板变化会自动同步。
5. 在“剪贴板”页可查看最近同步摘要；在“设置”页可调整外观、启动项（开机自启 / 静默启动）、同步能力与关窗行为。
6. 开启**静默启动**时，启动后仅显示托盘，首次点托盘打开主界面（冷启动 WebView）；再次打开通常更快（窗口 hide 后保留实例）。
7. 关闭主窗口时默认**收起到托盘**；可在设置中改为**直接退出**。连接流程详见 [`Docs/CONNECTION.md`](./Docs/CONNECTION.md)。

## 前端页面

- **剪贴板页**：展示同步历史（文本 / 图片 / 文件摘要）、时间、大小与来源
- **设备页**：展示局域网设备、连接状态、发起连接 / 断开连接入口
- **设置页**：外观、启动、应用行为、连接、同步、剪贴板展示等分组设置

## Tauri 命令

前端通过 `invoke` 调用 Rust 命令；完整列表见 `Apps/src-tauri/src/lib.rs` 中的 `generate_handler!`。按领域归纳如下：

| 领域 | 代表命令 |
|------|----------|
| Shell / 窗口 | `get_shell_bootstrap`、`get_shell_deferred`、`notify_main_ui_ready` |
| 状态与配对 | `get_status`、`get_pairing_code`、`rotate_pairing_code`、`end_pairing_session`、`pair`、`submit_pairing_code`、`submit_responder_pairing_code` |
| 连接 | `get_lan_devices`、`refresh_lan_devices`、`connect_lan`、`accept_connection`、`reject_connection`、`timeout_incoming_connection`、`disconnect`、`disconnect_peer`、`get_pending_connection_request`、`get_connected_peer(s)` |
| 设备信任 | `get_trusted_peers`、`remove_trusted_peer`、`set_peer_auto_accept` |
| 剪贴板 | `get_clipboard_history`、`copy_clipboard_history_entry`、`send_clipboard_history_entry`、`clear_clipboard_history`、`resolve_history_thumbnail`、`get_clipboard_settings`、`save_clipboard_settings` |
| 设置 | `get_ui_settings`、`save_ui_settings`、`get_startup_settings`、`save_startup_settings`、`get_app_behavior_settings`、`save_app_behavior_settings`、`get_connection_settings`、`save_connection_settings`、`get_sync_settings`、`save_sync_settings`、`pick_sync_files_save_dir`、`save_sync_files_save_dir` |

## 项目结构

```text
planarclip/
├── package.json             # workspace 根：转发脚本
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
├── Docs/                    # UI 规范、连接流程等产品文档
├── Apps/
│   ├── package.json
│   ├── index.html
│   ├── src/
│   │   ├── main.tsx
│   │   ├── app/
│   │   │   ├── components/
│   │   │   │   ├── common/      # 通用小组件
│   │   │   │   ├── ui/          # UI 规范薄组件
│   │   │   │   ├── layout/      # 侧栏与右侧概览面板
│   │   │   │   ├── overlays/    # 配对弹层、入站连接确认
│   │   │   │   └── pages/       # 剪贴板 / 设备 / 设置页面
│   │   │   ├── constants/       # 主题常量
│   │   │   ├── hooks/           # 桌面桥接、配对流程、主题状态
│   │   │   ├── utils/           # 消息、设备、时间、设置等工具
│   │   │   ├── App.tsx          # 前端主装配
│   │   │   └── types.ts         # 前端类型定义
│   │   └── styles/              # 全局样式、主题样式
│   └── src-tauri/
│       ├── src/
│       │   ├── clipboard/       # 剪贴板监听与历史摘要
│       │   ├── crypto/          # 密钥生成与指纹能力
│       │   ├── network/         # 局域网发现、连接与传输
│       │   ├── storage/         # 本地 JSON 配置持久化
│       │   ├── sync/            # 同步引擎
│       │   ├── tray/            # 托盘菜单与行为
│       │   ├── lib.rs           # Tauri 命令、状态管理、应用装配
│       │   └── main.rs          # Rust 入口
│       ├── tauri.conf.json      # Tauri 窗口与打包配置（主窗 create: false 等）
│       └── tauri.dev.conf.json  # 开发合并配置（与静默启动对齐）
```

## 配置文件

配置文件默认保存为 `planarclip_config.json`。

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%/planarclip_config.json` |
| macOS | `~/Library/Application Support/planarclip_config.json` |
| Linux | `~/.config/planarclip_config.json` |

当前会保存的核心配置包括：

- 设备名称、密钥对、TCP 端口、局域网开关
- 熟悉设备（`trusted_peers`）及入站是否自动接受
- UI 明暗模式、主题色
- 开机自启（`launch_at_startup`）、静默启动（`silent_start`）
- 关窗行为（`close_window_action`：`tray` / `exit`）、系统通知开关
- 自动连接熟悉设备、剪贴板自动同步、图片 / 文件同步及大小上限、接收文件保存目录
- 剪贴板历史条目与展示条数、视图模式（`list` / `grid`）

## 当前状态

| 模块 | 状态 |
|------|------|
| 文本剪贴板监听与同步 | ✅ 已实现 |
| 局域网设备发现 | ✅ 已实现 |
| 配对码连接确认 | ✅ 已实现 |
| 会话级配对码倒计时与轮换 | ✅ 已实现 |
| 托盘与窗口驻留 | ✅ 已实现 |
| 外观设置持久化 | ✅ 已实现 |
| 剪贴板历史摘要展示 | ✅ 已实现 |
| 图片同步 | ✅ Windows 已验证（inline / 大图分块 / 资源管理器粘贴，2026-06-29） |
| 文件同步 | ✅ Windows 已验证（单文件 + 13 文件批次 + 资源管理器粘贴，2026-06-29） |
| 入站连接等系统通知 | ✅ 已实现（设置中可关闭） |
| 静默启动 + 冷启动首屏优化 | ✅ 2026-07（无 WebView 预暖） |

## 项目文档

| 文件 | 用途 |
|------|------|
| [`README.md`](./README.md) | 产品介绍、安装运行、能力边界与配置文件说明（面向开发者与用户） |
| [`Docs/UI_GUIDE.md`](./Docs/UI_GUIDE.md) | **项目级 UI 规范**：文案与交互、设计令牌、组件模式、动画、浮层与 Figma 对照 |
| [`Docs/CONNECTION.md`](./Docs/CONNECTION.md) | **连接与配对**产品流程（熟悉 / 信任、设备分区、配对场景） |
| [`AGENTS.md`](./AGENTS.md) | 统一编码代理说明：协作约定、构建命令、UI 摘要与链接、GitNexus 与 MCP 约定（面向 Cursor / Claude Code 等 AI 工具） |
| [`CLAUDE.md`](./CLAUDE.md) | 指向 `AGENTS.md` 的 Claude Code 入口（内容已合并，不再单独维护） |
| [`Plans/`](./Plans/) | 功能方案讨论与执行记录 |

## 说明

- 项目当前以桌面端体验为主，若只运行 Web 预览，连接相关能力会显示为预览态提示。
- 若需要体验完整链路，请使用 `pnpm dev` 启动 Tauri 桌面应用。
- 使用 AI 编码工具协作时，请优先阅读 [`AGENTS.md`](./AGENTS.md)；做前端 UI 改动时另读 [`Docs/UI_GUIDE.md`](./Docs/UI_GUIDE.md)。
