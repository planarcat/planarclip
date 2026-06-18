# CLAUDE.md

本文件为在本仓库中协作的编码助手提供上下文与工作约定。

## 构建与运行

所有命令在 `Apps/planarclip/` 目录下执行：

```bash
pnpm exec vp install # 安装依赖
pnpm exec vp dev     # 启动前端开发服务器 (localhost:1420)
pnpm exec vp check   # 运行格式、lint、类型检查
pnpm exec vp build   # 前端生产构建
npx tauri dev        # 启动 Tauri 开发模式（含 Rust 后端）
npx tauri build      # 生产构建
```

也可以继续使用项目脚本别名：`pnpm dev`、`pnpm check`、`pnpm build`。前端开发服务器端口为 1420，HMR 端口为 1421。`tauri.conf.json` 已通过 `beforeDevCommand` / `beforeBuildCommand` 调用 `pnpm exec vp dev` 与 `pnpm exec vp build`。

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

- 前端命令已统一切换到 Vite+ (`vp`)。
- Tauri 的 `beforeDevCommand` / `beforeBuildCommand` 已改为 `pnpm exec vp dev` / `pnpm exec vp build`。
- `cargo build`、`npx tauri dev`、`npx tauri build` 在用户本机均已验证通过。
