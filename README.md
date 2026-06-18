# PlanarClip

PlanarClip 是一个使用 **Tauri 2 + TypeScript + Rust** 构建的跨设备剪贴板同步桌面应用。

当前版本聚焦在：

- 本地剪贴板变更监控
- 基于 WebSocket 信令 / 局域网直连的设备通信
- 配对码确认与可信设备持久化
- 系统托盘入口与轻量状态 UI

## 技术栈

- **桌面框架**: Tauri 2
- **前端**: TypeScript + Vite+
- **后端**: Rust + Tokio
- **发现/连接**: WebSocket 信令、mDNS、局域网 TCP 直连
- **剪贴板**: arboard
- **加密基础**: x25519-dalek + blake3

## 安装与开发

项目默认使用 **pnpm** 作为唯一包管理器。

### 安装依赖

在 `Apps/planarclip/` 目录下执行：

```bash
pnpm install
```

### 开发模式

```bash
# 仅启动前端开发服务器（localhost:1420）
pnpm dev

# 运行统一检查流程（格式、lint、类型检查）
pnpm check

# 启动完整 Tauri 应用（含 Rust 后端）
pnpm tauri dev
```

如需直接调用底层命令，可使用：`pnpm exec vp dev`、`pnpm exec vp check`。

### 生产构建

```bash
pnpm build       # 前端生产构建
pnpm tauri build # 生成安装包（Windows: NSIS，macOS: DMG）
```

## 使用方式

- 启动应用后，界面会显示当前设备的短配对码。
- 若使用信令服务器模式，可输入房间号与另一台设备加入同一房间。
- 若使用局域网直连模式，可从局域网设备列表中选择设备发起连接。
- 当目标设备不是已信任设备时，需要输入或确认 6 位配对码。
- 建立连接后，文本剪贴板内容会自动同步。

## 项目结构

```text
Apps/planarclip/
├── src/
│   └── main.ts              # 前端入口，连接状态、配对与局域网设备列表 UI
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Rust 入口
│   │   ├── lib.rs           # Tauri 应用组装、状态管理、Tauri 命令
│   │   ├── clipboard/       # 剪贴板监控与快照类型
│   │   ├── crypto/          # X25519 密钥对生成与指纹
│   │   ├── network/         # 信令连接、局域网发现与连接管理
│   │   ├── sync/            # 同步引擎与去重集合
│   │   ├── storage/         # AppConfig JSON 持久化
│   │   └── tray/            # 系统托盘菜单
│   └── tauri.conf.json      # 窗口、托盘与构建配置
└── index.html               # 前端入口（内联深色主题 CSS）
```

## 配置文件位置

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%/planarclip_config.json` |
| macOS | `~/Library/Application Support/planarclip_config.json` |
| Linux | `~/.config/planarclip_config.json` |

## 当前开发状态

| 功能 | 状态 |
|------|------|
| 剪贴板监控与变更检测 | ✅ 完成 |
| X25519 密钥对生成与设备指纹 | ✅ 完成 |
| 系统托盘与窗口管理 | ✅ 完成 |
| 前端基础 UI | ✅ 完成 |
| WebSocket 信令客户端 | ✅ 完成 |
| 局域网设备发现 | ✅ 完成 |
| 局域网直连握手 | ✅ 完成 |
| 配对码确认 | ✅ 完成 |
