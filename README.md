# PlanarClip

PlanarClip 是一个基于 **Tauri 2 + React 18 + TypeScript + Rust** 构建的跨设备剪贴板同步桌面应用。

当前版本已经完成桌面端主流程：

- 文本剪贴板监听与会话级历史摘要
- 局域网设备自动发现与直连
- 6 位配对码确认、可信设备持久化
- 托盘驻留、窗口显隐与基础桌面交互
- 外观设置持久化（明暗模式 / 主题色）

## 当前能力

### 已实现

- 剪贴板文本自动同步
- 局域网设备发现与连接
- 配对码验证与连接请求确认
- 最近 12 条文本同步历史展示
- 设备页、剪贴板页、设置页三栏桌面 UI
- 系统托盘入口与关闭窗口后驻留后台
- 外观设置保存到本地配置文件

### 当前限制

- 当前仅支持 **文本** 同步，图片 / 文件尚未接入
- 浏览器预览模式只能查看 UI，连接能力需在桌面应用中体验
- 默认局域网直连端口为 `19876`
- 项目里仍保留部分网络模块演进空间，但当前用户主路径以桌面端局域网连接为主

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

项目默认使用 **pnpm** 作为包管理器，应用主体位于 `Apps/planarclip/`。

## 安装与运行

在 `Apps/planarclip/` 目录下执行：

```bash
pnpm install
```

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

1. 启动桌面应用后，首页会显示当前设备的 6 位配对码。
2. 在“设备”页可查看已发现的局域网设备，并直接发起连接。
3. 若对方主动请求连接，应用会弹出连接确认弹层。
4. 建立连接后，文本剪贴板变化会自动同步。
5. 在“剪贴板”页可查看最近同步摘要，在“设置”页可调整主题外观。
6. 关闭主窗口时应用默认收回托盘，不会直接退出进程。

## 前端页面

- **剪贴板页**：展示文本同步历史、时间、大小与来源摘要
- **设备页**：展示局域网设备、连接状态、发起连接 / 断开连接入口
- **设置页**：保存背景模式与主题色，并展示当前同步能力边界

## Tauri 命令

当前前端主要通过以下命令与 Rust 层通信：

- `get_status`
- `get_pairing_code`
- `get_ui_settings`
- `save_ui_settings`
- `get_clipboard_history`
- `get_lan_devices`
- `pair`
- `connect_lan`
- `submit_pairing_code`
- `reject_connection`
- `disconnect`

## 项目结构

```text
Apps/planarclip/
├── src/
│   ├── app/
│   │   ├── components/
│   │   │   ├── common/      # 通用小组件
│   │   │   ├── layout/      # 侧栏与右侧概览面板
│   │   │   ├── overlays/    # 配对弹层
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
│   │   ├── clipboard/       # 剪贴板监听与历史摘要
│   │   ├── crypto/          # 密钥生成与指纹能力
│   │   ├── network/         # 局域网发现、连接与传输
│   │   ├── storage/         # 本地 JSON 配置持久化
│   │   ├── sync/            # 同步引擎
│   │   ├── tray/            # 托盘菜单与行为
│   │   ├── lib.rs           # Tauri 命令、状态管理、应用装配
│   │   └── main.rs          # Rust 入口
│   └── tauri.conf.json      # Tauri 窗口与打包配置
└── package.json
```

## 配置文件

配置文件默认保存为 `planarclip_config.json`。

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%/planarclip_config.json` |
| macOS | `~/Library/Application Support/planarclip_config.json` |
| Linux | `~/.config/planarclip_config.json` |

当前会保存的核心配置包括：

- 设备名称
- 密钥对
- 最近配对设备 / 已信任设备
- TCP 端口
- 局域网开关
- UI 明暗模式
- UI 主题色

## 当前状态

| 模块 | 状态 |
|------|------|
| 文本剪贴板监听与同步 | ✅ 已实现 |
| 局域网设备发现 | ✅ 已实现 |
| 配对码连接确认 | ✅ 已实现 |
| 托盘与窗口驻留 | ✅ 已实现 |
| 外观设置持久化 | ✅ 已实现 |
| 剪贴板历史摘要展示 | ✅ 已实现 |
| 图片同步 | 🚧 未实现 |
| 文件同步 | 🚧 未实现 |
| 桌面通知提醒 | 🚧 未实现 |

## 说明

- 项目当前以桌面端体验为主，若只运行 Web 预览，连接相关能力会显示为预览态提示。
- 若需要体验完整链路，请使用 `pnpm dev` 启动 Tauri 桌面应用。
