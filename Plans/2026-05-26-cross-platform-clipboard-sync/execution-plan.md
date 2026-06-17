---

## status: draft
generated: 2026-05-26
based_on:
  - 01-cross-platform-framework-selection.md
  - 02-performance-memory-ranking.md
  - 03-cross-device-sync-p2p-architecture.md
  - 04-tauri-vs-rust-native-shell.md
  - 05-cross-platform-testing-strategy.md
  - 06-pending-decisions-resolved.md
  - 07-foreseeable-challenges.md
  - 08-mvp-simplification.md

# PlanarClip — 待执行方案

> 生成时间: 2026-05-26 · 状态: 待审阅

---

## 1. 需求概述

构建跨平台剪贴板同步应用 **PlanarClip**：


| 需求  | 说明                            |
| --- | ----------------------------- |
| 平台  | Windows + macOS               |
| 常驻  | 系统托盘（Win 任务栏 / Mac 菜单栏）       |
| 内容  | 文字、图片、文件（含大文件）                |
| 设备  | Win↔Win、Win↔Mac、Mac↔Mac，多设备同时 |
| 安全  | E2E 加密，数据不经过第三方服务器            |


---

## 2. 技术决策总表


| #   | 决策项     | 选择                                                         | 理由                                           | 来源        |
| --- | ------- | ---------------------------------------------------------- | -------------------------------------------- | --------- |
| 1   | 客户端框架   | **Tauri 2**                                                | 空载 10–30MB，远优于 Electron 150MB+；Rust 核心可零修改复用 | 第 1、2、4 轮 |
| 2   | 前端      | **单页 HTML**（MVP）；React 19 + TS（Phase 4）                    | 配对页只需输入框，Dashboard 只需状态，先轻后重                 | 第 4、8 轮   |
| 3   | 网络架构    | **P2P 直连** (WebRTC) + WS 信令                                | 大文件不经过服务器，局域网 70–100MB/s+，隐私性强               | 第 3 轮     |
| 4   | 信令服务器   | **公共实例**（MVP）；独立 Axum（Phase 4）                             | 用户零配置即可开始使用                                  | 第 3、6、8 轮 |
| 5   | 渐进路线    | 文本同步(MVP) → 安全/持久化 → 文件/多设备 → 打磨                           | 快交付 + 可进化                                    | 第 3、8 轮   |
| 6   | 加密      | **WebRTC DTLS**（MVP）；Noise IK + ChaCha20-Poly1305（Phase 2） | WebRTC 内置 DTLS 已加密传输；Phase 2 补应用层加密          | 第 8 轮     |
| 7   | 剪贴板     | arboard                                                    | 唯一维护的跨平台 crate，被 1Password 使用                | —         |
| 8   | 本地存储    | **JSON 文件**（MVP）；SQLite (rusqlite)（Phase 2）                | MVP 历史记录和配置用 JSON 足够；后续迁 SQLite              | 第 8 轮     |
| 9   | 测试策略    | 单机双实例(日常) + CI 双平台(Phase 2) + 真机(里程碑)                      | 日常无需发版到另一台设备                                 | 第 5、8 轮   |
| 10  | 设备配对    | **6 位数字配对码**                                               | 桌面端无摄像头，扫码体验差；移动端再加 QR                       | 第 6 轮     |
| 11  | 信令服务器   | **公共实例，零配置**                                               | 用户安装后输入配对码即可使用，无需配置服务器                       | 第 6 轮     |
| 12  | 剪贴板安全过滤 | **推迟到未来版本**                                                | MVP 不处理密码管理器过滤                               | 第 6 轮     |
| 13  | 大文件上限   | **硬编码 2GB**                                                | 未来版本加配置项                                     | 第 6 轮     |
| 14  | 离线策略    | **仅保留最新一条**                                                | 符合剪贴板"当前内容"的语义                               | 第 6 轮     |


---

## 3. 架构

### 3.1 系统拓扑

```
                    ┌─────────────────┐
                    │  Signalling Srv  │  (WS relay only)
                    └──┬────┬────┬────┘
                       │    │    │
        ┌──────────────┼────┼────┼──────────────┐
        │              │    │    │              │
   ┌────▼────┐    ┌────▼────▼────▼────┐    ┌────▼────┐
   │ Device A │    │    Device B/C    │    │ Device D │
   │  (Win)   │    │   (Win / Mac)    │    │  (Mac)   │
   └────┬─────┘    └────┬──────┬──────┘    └────┬─────┘
        │               │      │               │
        └───────────────┼──────┼───────────────┘
                        │ WebRTC P2P (encrypted)
                        │ 70–100 MB/s (LAN)
```

### 3.2 客户端数据流（MVP）

```
┌─────────────────────────────────────────────┐
│              Tauri Rust Backend              │
│                                              │
│  Clipboard Monitor ──→ Sync Engine ──→ WebRTC(DTLS) ──→ Peer
│    (500ms poll)         (orchestrate)   (built-in)       │
│         │                    │              │             │
│         ▼                    ▼              ▼             │
│      arboard            JSON files     Signalling(WS)     │
│    (OS clipboard)    (history/config)  (public relay)     │
│                                              │            │
└──────────────────────────────────────────────┼────────────┘
                                               │
┌──────────────────────────────────────────────┼────────────┐
│              单页 HTML Frontend              │            │
│                                              │            │
│  配对码输入 + 连接状态 + 最近同步预览        ◀────────────┘
└──────────────────────────────────────────────┘
```

### 3.3 消息协议（MVP）

```
Envelope {
    message_id:   Uuid
    timestamp:    i64 (Unix millis)
    content_type: Text(1) | Heartbeat(5)
    payload:      Vec<u8> (plain JSON over DTLS)
}
```

> Phase 2 加入 Noise 应用层加密后，payload 层叠加密。
> Phase 2 加入图片/文件类型后扩展 content_type 枚举。

### 3.4 传输策略（MVP）


| 类型       | 处理                  |
| -------- | ------------------- |
| 文本（任意长度） | `clipboard` 通道，全量同步 |


> 图片、文件、大文件阈值分界 → Phase 2–3

---

## 4. 项目结构

```
planarclip/
├── Plans/
│   ├── 2026-05-26-cross-platform-clipboard-sync/
│   │   ├── 01–08-*.md              # 讨论记录
│   │   └── execution-plan.md       # 本文件
│   └── deepseek-conversation-original.md
├── src/                            # 前端
│   ├── index.html                  # 单页 HTML（MVP）
│   ├── main.ts                     # 前端入口
│   └── app.ts                      # 基础组件
├── src-tauri/                      # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── lib.rs                  # 启动入口
│       ├── clipboard/
│       │   ├── monitor.rs          # 500ms 轮询 + blake3 去重
│       │   └── types.rs            # ClipboardSnapshot enum
│       ├── sync/
│       │   ├── engine.rs           # 同步协调器
│       │   └── dedup.rs            # 回环抑制
│       ├── network/
│       │   ├── signalling.rs       # WS 信令客户端
│       │   └── webrtc.rs           # WebRTC 连接管理
│       ├── crypto/
│       │   └── keys.rs             # X25519 密钥对 + 简易存储
│       ├── storage/
│       │   └── json.rs             # JSON 文件读写
│       ├── tray/
│       │   └── menu.rs             # 托盘菜单
│       └── util/
│           └── hash.rs             # blake3
├── package.json
└── vite.config.ts
```

> Phase 2 加入: `commands/` · `peer.rs` · `noise.rs` · `db.rs` · `channels.rs`
> Phase 3 加入: `signalling-server/`
> Phase 4 加入: `pages/` · `components/` · `store/` · `hooks/`

---

## 5. 实现阶段

### Phase 1 · MVP — 文本同步

> 目标: 两台设备配对，文本剪贴板双向同步，托盘常驻
> 预计: **2–3 周**
> 执行策略: `plan-execution` 默认执行本 Phase

- Tauri 2 脚手架 + 单页 HTML 前端
- 剪贴板文本监控（arboard, 500ms 轮询, blake3 去重）
- 剪贴板文本写入（接收远程内容，自写入回环抑制）
- 信令连接（连接公共实例，room join/leave）
- 双向信令通道 + 消息协议（WS 中继剪贴板文本，MVP 替代 WebRTC）→ [第 9 轮](09-webrtc-implementation.md)
- 同步引擎集成（发送: 序列化→WS / 接收: 去重→写入剪贴板）
- X25519 密钥对生成 + 6 位配对码 + 密钥交换
- 密钥持久化（重启后配对码不变）
- 系统托盘图标 + 基础菜单（显示/退出）
- 前端：配对码输入 + 连接状态显示
- 前端：配对按钮接入后端命令
- 单机双实例开发测试

**MVP 不含：** SQLite（JSON 文件代替）、Noise 加密（WS WSS 代替）、图片/文件、CI、独立信令服务器

> 2026-05-27 更新: WebRTC P2P 推迟到 Phase 2+，MVP 用信令 WebSocket 直接中继文本（WSS 已提供传输加密）。详见第 9 轮讨论。

### Phase 2 · 安全与持久化

> 预计: **2–3 周**

- SQLite 存储（clipboard_entries + settings + peers + key_store）
- Noise IK 应用层加密 + ChaCha20-Poly1305 传输
- 图片同步（arboard::get_image, ≤5MB 全量传输）
- CI: GitHub Actions Win x64 + macOS ARM64 编译

### Phase 3 · 文件与多设备

> 预计: **3–4 周**

- 平台文件列表读取（Win: CF_HDROP / Mac: NSFilenamesPboardType）
- 大文件分块传输协议（FileList → FileChunk → FileAck, blake3 校验）
- 文件暂存目录 + 原子移动
- N 对等点 WebRTC 动态连接管理
- 断线自动重连（指数退避, max 30s）
- TURN 服务器回退
- 离线队列 + 冲突解决（last-writer-wins）

### Phase 4 · 打磨与发布

> 预计: **3–4 周**

- 完整 React 前端（Dashboard · History · Devices · Settings）
- mDNS LAN 自动发现
- 原生通知（tauri-plugin-notification）
- 暗色/亮色主题、键盘快捷键
- 独立信令服务器二进制（可选自部署）
- 剪贴板安全过滤（密码管理器进程名过滤）
- Windows NSIS/MSI + macOS DMG 安装包 + 签名
- 用户文档

---

## 6. 测试策略


| 层级     | 频率    | 方式                                                       |
| ------ | ----- | -------------------------------------------------------- |
| 单元测试   | 每次提交  | `cargo test` — 加密、序列化、去重、协议解析                            |
| 双实例集成  | 日常开发  | `PLANARCLIP_MODE=send/recv` + `--data-dir` + `--port` 隔离 |
| CI 双平台 | 每次 PR | GitHub Actions Win x64 + macOS ARM64 编译                  |
| 真机 E2E | 里程碑   | 两台物理机 Win↔Mac 完整流程                                       |


> 详见 [第 5 轮讨论](05-cross-platform-testing-strategy.md)

---

## 7. 关键依赖

### Phase 1 · MVP

**Rust:** `tauri 2` (tray-icon) · `arboard` · `webrtc` · `tokio-tungstenite` · `blake3` · `rand` · `x25519-dalek` · `serde` / `serde_json` · `uuid` · `tokio`

**前端:** 单页 HTML + 原生 JS/TS（无需框架）

### Phase 2 加入

`snow` · `chacha20poly1305` · `rusqlite` (bundled) · `keyring` · `bincode` · `image`

### Phase 3 加入

`chrono` · `directories` · `thiserror` · `tracing`

### Phase 4 加入

**前端:** `react 19` · `@tauri-apps/api 2` · `zustand` · `@tanstack/react-router` · `tailwindcss 4`
**信令服务器:** `axum` (ws) · `dashmap` · `clap` · `tower-http`

---

## 8. 参考


| 轮次  | 文件                                                                                   | 主题             |
| --- | ------------------------------------------------------------------------------------ | -------------- |
| 1   | [01-cross-platform-framework-selection.md](01-cross-platform-framework-selection.md) | 跨平台框架选型        |
| 2   | [02-performance-memory-ranking.md](02-performance-memory-ranking.md)                 | 性能/内存排行        |
| 3   | [03-cross-device-sync-p2p-architecture.md](03-cross-device-sync-p2p-architecture.md) | P2P 网络架构       |
| 4   | [04-tauri-vs-rust-native-shell.md](04-tauri-vs-rust-native-shell.md)                 | Tauri vs 原生壳   |
| 5   | [05-cross-platform-testing-strategy.md](05-cross-platform-testing-strategy.md)       | 测试策略           |
| 6   | [06-pending-decisions-resolved.md](06-pending-decisions-resolved.md)                 | 五项待决议事项拍板      |
| 7   | [07-foreseeable-challenges.md](07-foreseeable-challenges.md)                         | 可预见的开发难题       |
| 8   | [08-mvp-simplification.md](08-mvp-simplification.md)                                 | MVP 精简方案       |
| —   | [../deepseek-conversation-original.md](../deepseek-conversation-original.md)         | 原始 DeepSeek 对话 |


