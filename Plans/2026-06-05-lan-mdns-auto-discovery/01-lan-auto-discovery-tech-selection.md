# 第 1 轮：LAN 自动发现的技术选型与架构设计

> 时间: 2026-06-05

## 用户提问（原文）

开一个新主题，实现局域网内自动发现的交互机制

## 背景与分析

### 当前架构回顾

当前 PlanarClip 的网络层架构：

```
[前端 UI] → pair(code) → [lib.rs]
                              ↓
                     ConnectionManager::connect()
                              ↓
                     signalling::connect(ws://host/room/{code}/peer/{id})
                              ↓
                     WebSocket 双向通道
                              ↓
                SignalMessage::Clipboard { payload, hash }
```

**核心问题：** 两台设备在同一个局域网时，剪贴板数据仍然要绕一圈信令服务器（`ws://localhost:8765`），即使两台机器可以直接通信。这带来：
- 不必要的延迟（本地 → 服务器 → 本地）
- 依赖外部服务（信令服务器宕机则完全不可用）
- 局域网带宽被浪费

### mDNS 是什么

mDNS（Multicast DNS）是零配置网络发现协议，工作方式：
1. 设备 A 在局域网广播 "有人提供 `_planarclip._tcp` 服务吗？"
2. 设备 B 响应 "我！我的 IP 是 192.168.1.5，端口是 9876"
3. 设备 A 直接连接 `192.168.1.5:9876` 建立 TCP 连接

不需要任何中心服务器，路由器也不需要特殊配置。

### Rust mDNS 库选型

| 库 | 优势 | 劣势 |
|:---|:---|:---|
| **`mdns-sd`** | 纯 Rust，跨平台，API 简洁，同时支持注册和发现 | Windows 支持需要开启特定 feature |
| `zeroconf` | 基于 Avahi/Bonjour 封装，功能完善 | Linux 依赖 avahi-daemon，Windows 依赖 Bonjour 服务 |
| `libmdns` | 纯 Rust，异步友好 | 维护不活跃，文档较少 |
| `dns-sd` | Apple 官方 Bonjour 绑定 | 仅 macOS/iOS |

**推荐 `mdns-sd`**：纯 Rust 实现，不依赖系统服务，跨平台（Win/Mac/Linux），API 直观，与现有 tokio 异步架构兼容。

### 架构集成方案

新架构应该是**双模式**的：

```
                     ┌─ 局域网可用？─┐
                     ↓               ↓
              mDNS 发现         信令服务器
              TCP 直连          WebSocket 中继
                     ↓               ↓
              统一的消息协议（复用现有 SignalMessage）
```

**核心思路：**
1. 启动时同时进行 mDNS 广播（宣告自己）和 mDNS 浏览（发现别人）
2. 发现局域网对端后，直接建立 TCP 连接
3. TCP 连接上使用与信令相同的 `SignalMessage` 协议
4. 如果 mDNS 未发现任何设备，回退到手动输入配对码 + 信令服务器模式

### 需要讨论的关键决策

1. **mDNS 服务类型命名** — 建议 `_planarclip._tcp.local.`
2. **TCP 端口选择** — 固定端口 vs 动态端口（固定简单但有冲突风险）
3. **安全模型** — ✅ 已在[第 2 轮](02-lan-connection-flow-design.md)确定：被申请方生成临时随机 6 位配对码，发起方输入验证（带外验证模式）
4. **发现后的配对确认** — ✅ 已在[第 2 轮](02-lan-connection-flow-design.md)确定：被申请方弹出配对码 → 发起方输入 → 被申请方验证通过后建立连接（蓝牙式配对）
5. **与现有 `pair` 命令的关系** — ✅ 已在[第 2 轮](02-lan-connection-flow-design.md)确定：共存，局域网用临时随机码，远程用指纹提取码

## 建议与回答

### 推荐方案概述

新增 `network/discovery.rs` 模块，使用 `mdns-sd` 库实现：

**注册端（服务宣告）：**
```rust
// 启动时注册 _planarclip._tcp 服务
// 宣告: 设备名、端口号、peer_id（指纹的前 6 位）
let service_info = ServiceInfo::new(
    "_planarclip._tcp.local.",
    &device_name,       // 实例名 = 设备名
    &hostname,          // 本机主机名
    local_ip,           // 本机 IP
    port,               // TCP 监听端口
    &[("peer_id", &fingerprint_short)],
)?;
```

**浏览端（设备发现）：**
```rust
// 持续浏览 _planarclip._tcp 服务
// 发现新服务 → 通知前端更新设备列表
// 服务消失 → 从列表移除
let discovery = Discovery::new("_planarclip._tcp.local.")?;
```

### 前端交互建议

在现有 UI 中增加一个 **"局域网设备"** 区域：

```
┌─────────────────────────────┐
│  PlanarClip                 │
│                             │
│  📡 局域网设备              │
│  ┌─────────────────────┐   │
│  │ 🖥 My-Desktop       │   │  ← 自动发现，点击连接
│  │ 💻 My-Laptop        │   │
│  └─────────────────────┘   │
│                             │
│  ── 或 ──                  │
│                             │
│  配对码: [870921]           │  ← 手动输入（远程连接）
│  [连接]                     │
└─────────────────────────────┘
```

### 实现分阶段

**Phase 1: 核心发现 + 直连**
- [ ] 添加 `mdns-sd` 依赖
- [ ] `network/discovery.rs` — mDNS 注册 + 浏览
- [ ] `network/direct.rs` — TCP 直连（复用 SignalMessage 协议）
- [ ] 前端：显示发现的设备列表
- [ ] 前端：点击设备触发连接

**Phase 2: 完善体验**
- [ ] 自动重连（设备重新上线时）
- [ ] 连接状态指示（已连接/连接中/离线）
- [ ] 多设备管理（未来 N 对等点场景）

## 本轮要点

- 推荐使用 `mdns-sd` 纯 Rust 库，不依赖系统服务
- 双模式架构：mDNS 优先，信令服务器作为远程回退
- 直连 TCP 通道复用现有 `SignalMessage` 协议
- 前端增加"局域网设备"自动发现区域
- 发现后仍需用户点击确认连接（而非静默自动连接）

> 更新: 2026-06-07 — 第 2 轮讨论细化了连接流程，明确了蓝牙式配对验证机制，见 [02-lan-connection-flow-design.md](02-lan-connection-flow-design.md)
