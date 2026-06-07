# PlanarClip

跨设备剪贴板同步工具，基于 Tauri v2 构建。通过 WebRTC 点对点连接在两台设备之间实时同步剪贴板文本内容，配合信令服务器进行设备发现和配对。

## 功能

- **实时剪贴板监控** — 每 500ms 轮询，BLAKE3 哈希去重
- **加密配对** — X25519 密钥交换，6 位数字配对码
- **P2P 数据传输** — WebRTC 直连，低延迟
- **系统托盘驻留** — 后台运行，左键点击切换窗口显示
- **自写检测** — 防止同步循环
- **跨平台** — Windows / macOS

## 技术栈

| 层 | 技术 |
|---|------|
| 桌面框架 | Tauri v2 |
| 后端 | Rust (Tokio 异步运行时) |
| 前端 | Vite v6 + TypeScript，内联 CSS |
| 剪贴板 | arboard |
| 加密 | X25519 (x25519-dalek), BLAKE3 |
| 信令 | WebSocket (tokio-tungstenite) |
| P2P | WebRTC |

## 前置条件

- [Rust](https://www.rust-lang.org/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- 平台构建依赖（Windows: Visual Studio Build Tools / macOS: Xcode Command Line Tools）

## 构建与运行

```bash
cd Apps/planarclip

# 开发模式
npm install
npx tauri dev

# 生产构建
npx tauri build
```

信令服务器默认连接 `ws://localhost:8765`。

## 项目结构

```
Apps/planarclip/
├── src/
│   └── main.ts                 # 前端入口，连接状态展示、配对 UI
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             # Rust 入口
│   │   ├── lib.rs              # Tauri 组装：状态管理、Tauri 命令、系统托盘
│   │   ├── clipboard/
│   │   │   ├── monitor.rs      # ClipboardMonitor：轮询 + BLAKE3 哈希去重
│   │   │   └── types.rs        # ClipboardSnapshot 枚举
│   │   ├── crypto/
│   │   │   └── keys.rs         # X25519 密钥对生成、配对码派生
│   │   ├── network/
│   │   │   ├── signalling.rs   # WebSocket 信令客户端
│   │   │   └── webrtc.rs       # WebRTC 连接管理
│   │   ├── sync/
│   │   │   ├── engine.rs       # SyncEngine：变更转发
│   │   │   └── dedup.rs        # DedupStore：环形去重集合
│   │   ├── storage/
│   │   │   └── json.rs         # 本地 JSON 配置持久化
│   │   ├── util/
│   │   │   └── hash.rs         # BLAKE3 哈希辅助函数
│   │   └── tray/
│   ├── tauri.conf.json         # Tauri 配置
│   └── capabilities/
└── index.html                  # HTML 入口，内联深色主题 CSS
```

## 数据流

```
剪贴板轮询 → BLAKE3 哈希 → broadcast channel → SyncEngine → WebRTC → 对端
                                                                    ↓
                             对端 SyncEngine ← broadcast channel ← 接收
```

自写检测通过全局 `AtomicBool` 标志实现，写入剪贴板时跳过本轮变更检测，防止同步循环。

## 配对流程

1. 设备 A 生成 X25519 密钥对，派生出 6 位配对码
2. 设备 B 输入设备 A 的配对码
3. 双方通过信令服务器建立 WebRTC 连接
4. 连接建立后，剪贴板内容开始实时同步

## 开发状态

- [x] 剪贴板监控和变更检测
- [x] X25519 密钥对生成和配对码
- [x] 系统托盘和窗口管理
- [x] 前端基础 UI
- [x] WebSocket 信令客户端
- [x] WebRTC P2P 连接管理
- [ ] 端到端加密传输
- [ ] 多格式剪贴板支持（图片、文件）
