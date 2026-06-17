# PlanarClip

跨设备剪贴板同步工具，基于 Tauri v2 构建。通过 WebSocket 信令服务器完成设备配对，使用 6 位数字配对码建立连接，实时同步两台设备之间的剪贴板文本内容。

## 功能特性

- **实时同步** — 剪贴板变更在 500ms 内检测并推送到对端设备
- **6 位配对码** — 蓝牙配对风格的设备连接体验，无需账号注册
- **防循环写入** — 自写检测机制，程序自身写入剪贴板时不触发同步
- **去重保护** — 环形去重集合防止相同内容反复同步
- **系统托盘** — 后台常驻，左键点击托盘图标切换窗口显示/隐藏
- **持久化配置** — 密钥对和配对信息本地保存，重启后自动恢复

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri v2（Rust 后端 + WebView 前端） |
| 前端 | Vite v6 + TypeScript |
| 剪贴板 | arboard（跨平台 Rust 库） |
| 内容哈希 | BLAKE3 |
| 加密 | X25519 密钥交换（x25519-dalek） |
| 网络传输 | WebSocket（tokio-tungstenite） |
| 异步运行时 | Tokio |
| 本地存储 | JSON 配置文件 |

## 快速开始

### 前置依赖

- [Rust](https://rustup.rs/)（stable）
- [Node.js](https://nodejs.org/) v18+
- [Tauri CLI 前置依赖](https://tauri.app/start/prerequisites/)（平台相关）

### 安装依赖

在 `Apps/planarclip/` 目录下执行：

```bash
npm install
```

### 开发模式

```bash
# 仅启动前端开发服务器（localhost:1420）
npm run dev

# 启动完整 Tauri 应用（含 Rust 后端）
npx tauri dev
```

### 生产构建

```bash
npm run build        # TypeScript 编译 + Vite 构建
npx tauri build      # 生成安装包（Windows: NSIS，macOS: DMG）
```

## 使用方法

1. 在两台设备上分别启动 PlanarClip
2. 确保两台设备都能连接到同一个信令服务器（默认 `ws://localhost:8765`）
3. 在设备 A 上，记录窗口中显示的**我的配对码**（6 位数字）
4. 在设备 B 上，将设备 A 的配对码输入到**配对码输入框**，点击 Connect
5. 连接建立后，任一设备的剪贴板文本变更都会自动同步到对端

> 窗口默认隐藏，点击系统托盘图标可打开/关闭窗口。

## 项目结构

```
Apps/planarclip/
├── src/
│   └── main.ts              # 前端逻辑（连接状态、配对输入）
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Rust 入口
│   │   ├── lib.rs           # Tauri 应用组装、状态管理、Tauri 命令
│   │   ├── clipboard/       # 剪贴板监控与快照类型
│   │   ├── crypto/          # X25519 密钥对生成与配对码
│   │   ├── network/         # WebSocket 信令 + WebRTC 连接管理
│   │   ├── sync/            # 同步引擎与去重集合
│   │   ├── storage/         # AppConfig JSON 持久化
│   │   ├── tray/            # 系统托盘菜单
│   │   └── util/            # BLAKE3 哈希辅助
│   └── tauri.conf.json      # 窗口 380×520、默认隐藏、托盘图标
└── index.html               # 前端入口（内联深色主题 CSS）
```

## 配置文件位置

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\planarclip_config.json` |
| macOS | `~/Library/Application Support/planarclip_config.json` |
| Linux | `~/.config/planarclip_config.json` |

## 开发状态

| 功能 | 状态 |
|------|------|
| 剪贴板监控与变更检测 | ✅ 完成 |
| X25519 密钥对生成与配对码 | ✅ 完成 |
| 系统托盘与窗口管理 | ✅ 完成 |
| 前端基础 UI | ✅ 完成 |
| WebSocket 信令客户端 | ✅ 完成 |
| WebRTC P2P 数据传输 | 🚧 待实现 |
| 完整加密配对与双向同步 | 🚧 待实现 |
| 图片与文件同步 | 📋 规划中 |
| 多设备支持 | 📋 规划中 |

## 已知限制

- 当前 MVP 阶段配对码接受任意 6 位码并标记为已连接，完整的 X25519 加密握手尚未实现
- 仅支持文本内容同步，图片和文件同步待后续版本
- 信令服务器默认指向 `ws://localhost:8765`，生产部署需修改 `lib.rs` 中的 `SIGNALLING_SERVER` 常量

## License

MIT
