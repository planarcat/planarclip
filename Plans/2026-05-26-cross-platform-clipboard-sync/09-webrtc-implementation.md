# 第 9 轮：WebRTC 传输层实现方案

> 时间: 2026-05-27 19:30

## 用户提问（原文）

现在继续未完成的开发，目前应该还有webrtc没有实现。如果要规划WebRTC实现方案，那就进入方案设计模式，先给一个方案出来。哪个快速用哪个。

## 背景与分析

当前 MVP Phase 1 完成度：

| 模块 | 状态 | 备注 |
|------|------|------|
| 剪贴板监控 (500ms+BLAKE3) | ✅ 完成 | `clipboard/monitor.rs` |
| X25519 密钥对+6位配对码 | ✅ 完成 | `crypto/keys.rs` |
| 系统托盘+窗口管理 | ✅ 完成 | `lib.rs` |
| 信令客户端 | ⚠️ 仅接收 | `signalling.rs` 丢弃了 WS write 半边，无法发送 |
| WebRTC | ❌ 空 stub | `webrtc.rs` 只有一行注释 |
| 同步引擎 | ⚠️ 骨架 | `engine.rs` 收到变更只打日志，不传输 |
| DedupStore | ⚠️ 未集成 | 代码存在但未接入 SyncEngine |
| 密钥持久化 | ❌ 缺失 | 密钥只在内存，重启丢失 |
| 前端配对按钮 | ⚠️ 未接入 | `main.ts` 有 TODO，未调用 `invoke("pair")` |

代码探索发现的核心缺口：
1. `signalling.rs` — `connect()` 分割 WS 流后丢弃了 `write` 半边，没有发送路径
2. `webrtc.rs` — 空文件，只有注释 "MVP: stub"
3. `engine.rs` 第 28 行 — `// TODO: broadcast to connected peers via WebRTC`
4. `main.ts` 第 43 行 — `// TODO: initiate pairing via Tauri command`
5. `lib.rs` 的 `pair` 命令直接设 `connected = true`，没有任何网络连接逻辑
6. KeyPair 生成后不持久化，重启丢密钥

核心抉择：传输方式

| | 方案A: WS 中继 | 方案B: webrtc-rs |
|------|------|------|
| 新增依赖 | 无 | `webrtc` (ICE/STUN/DTLS/SCTP 全栈) |
| 实现周期 | 1-2 天 | 1-2 周 |
| 数据路径 | 经信令服务器 | P2P 直连 |
| 加密 | WSS 传输层 | DTLS 内建 |
| 编译时间 | 不变 | 显著增加 |

## 建议与回答

选择方案 A：用信令 WebSocket 直接中继剪贴板文本。零新增依赖，复用 `tokio-tungstenite`，快速跑通 MVP。

### 实现步骤

**Step 1 — 消息协议** `network/protocol.rs`（新文件）

```rust
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SignalMessage {
    #[serde(rename = "clipboard")]
    Clipboard { payload: String, hash: String },
}
```

**Step 2 — 信令客户端升级为双向** `network/signalling.rs`（重写）

`SignallingClient` 提供 `send()` 方法。内部 spawn 两个任务：
- 读 WS → parse JSON → 推入 `rx: UnboundedReceiver<SignalMessage>`
- 从内部 `tx` 读 → write 到 WS

**Step 3 — 连接管理器** `network/connection.rs`（重写 webrtc.rs）

```rust
pub struct ConnectionManager {
    signalling: SignallingClient,
    connected: Arc<Mutex<bool>>,
    clip_tx: broadcast::Sender<ClipboardSnapshot>,
    dedup: DedupStore,
}
```

- `connect(room_code)` — 连信令入房间
- `send_clipboard(snapshot)` — 序列化发送
- 接收循环 — parse Clipboard → dedup 检查 → SELF_WRITING=true → 写入本地剪贴板
- WS 断开 → connected = false

**Step 4 — 配对命令重写** `lib.rs`（修改）

`pair` 命令改为：取 peer_id → 初始化 ConnectionManager 连 room={code} → connected=true → 持久化 peer 信息。同时修复 KeyPair 持久化。

**Step 5 — 同步引擎集成** `sync/engine.rs`（修改）

SyncEngine 持有 `ConnectionManager`，收到本地剪贴板变更 → 发送；远端消息由 ConnectionManager 直接写入剪贴板（设 SELF_WRITING）。

**Step 6 — 前端修复** `main.ts`（修改）

- pairBtn → `invoke("pair", { code })`
- 启动时显示己方配对码
- lastSync 时间戳

### 涉及文件

| 操作 | 文件 |
|------|------|
| 新建 | `src-tauri/src/network/protocol.rs` |
| 重写 | `src-tauri/src/network/signalling.rs` |
| 重写 | `src-tauri/src/network/webrtc.rs` → `connection.rs` |
| 修改 | `src-tauri/src/network/mod.rs` |
| 修改 | `src-tauri/src/lib.rs` |
| 修改 | `src-tauri/src/sync/engine.rs` |
| 修改 | `src/main.ts` |

### 验证方式

1. `cargo check` 编译通过
2. 单机两实例测试：A 复制 → B 粘贴，B 复制 → A 粘贴
3. 重启验证密钥持久化
4. 前端状态正确显示

## 本轮要点

- MVP 不用 webrtc-rs，用信令 WebSocket 直传文本
- 零新增依赖，复用现有 tokio-tungstenite
- 6 步实现，涉及 7 个文件
- 配对码即信令房间号
- Phase 2 再切换到真正的 WebRTC P2P
