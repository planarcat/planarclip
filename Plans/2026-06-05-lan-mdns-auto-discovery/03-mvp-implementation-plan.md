# 第 3 轮：MVP 最简实现方案

> 时间: 2026-06-07
> 前置: [第 1 轮](01-lan-auto-discovery-tech-selection.md) | [第 2 轮](02-lan-connection-flow-design.md)

## MVP 目标

1. 两台设备 mDNS 互相发现
2. 设备 1 发起连接，设备 2 弹出配对码 → 设备 1 输入 → 连接建立
3. 设备 1 复制文本，设备 2 能收到（明文，局域网内）
4. 设备 1 重启后自动连上设备 2（已配对设备跳过配对码，无需用户操作）

## 核心决策汇总

对第 2 轮遗留的 9 个问题进行逐一决策：

| # | 问题 | 决策 | 理由 |
|:---|:---|:---|:---|
| 1 | TCP 端口 | **固定 19876，写入 AppConfig 可覆盖** | 简单；后续改动态端口只需改配置值 |
| 2 | 加密通道 | **MVP 不做加密**，握手阶段交换公钥，后续升级只需加帧类型 + DH | LAN 内可信任；公钥已交换，升级零破坏 |
| 3 | 双向同时连接 | **响应方优先：收到入站连接时，若自己也在向同一 peer 发起出站连接，则中止出站，转为响应方** | 无锁竞争，逻辑简单 |
| 4 | TCP 帧协议 | **`[1 字节帧类型][4 字节大端长度][payload]`**，类型 `0x00`=握手JSON，`0x01`=数据JSON，`0x02`=加密数据(预留) | 5 字节开销，线程安全分帧，可扩展 |
| 5 | 多设备 | **MVP 单连接**，新连接替换旧连接；`ConnectionHandle` 已是 clonable，后续改 `HashMap` 即可 | 最小改动 |
| 6 | 可被发现 | **应用运行即开启 mDNS**（配对码保护足够） | 最简 UX |
| 7 | 持久化重连 | **存储对端公钥到 `trusted_peers`，重连时匹配公钥 → 自动跳过配对码** | 配对过的设备无需再次肉眼验证 |
| 8 | 配对码过期 | **60s 过期后 TCP 连接关闭 + 前端弹窗自动消失**，发起方收到 AuthResult(success=false) 并提示"配对码已过期" | 同步双方状态 |
| 9 | 握手序列化 | **JSON（与 SignalMessage 一致）** | 复用 serde，调试友好 |

## 架构总览

```
┌─ 前端 UI ─────────────────────────────────────────────────────┐
│  [局域网设备列表]  [配对码弹窗]  [连接状态]                      │
└──────────────────────────┬─────────────────────────────────────┘
                           │ Tauri commands / events
┌──────────────────────────┴─────────────────────────────────────┐
│  lib.rs                                                       │
│  ┌──────────────────┐  ┌─────────────────────────────────┐    │
│  │ AppState (扩展)  │  │ ConnectionManager (新增方法)     │    │
│  │ - trusted_peers  │  │ - connect_lan(ip, port)         │    │
│  │ - pending_hs     │  │ - accept_incoming(tcp_stream)   │    │
│  └──────────────────┘  └──────────┬──────────────────────┘    │
│                                   │                            │
│  ┌────────────────────────────────┴──────────────────────┐    │
│  │                 Transport (抽象层)                     │    │
│  │  ┌──────────────┐  ┌──────────────────────────────┐   │    │
│  │  │ signalling   │  │ direct (新增)                 │   │    │
│  │  │ (WebSocket)  │  │ - TCP 帧编解码                │   │    │
│  │  └──────────────┘  │ - 握手状态机                   │   │    │
│  │                    │ - frame::read/write            │   │    │
│  │                    └──────────────────────────────┘   │    │
│  └──────────────────────────────────────────────────────┘    │
│                                   │                            │
│  ┌────────────────────────────────┴──────────────────────┐    │
│  │ discovery (新增)                                       │    │
│  │ - mDNS 注册 (_planarclip._tcp.local.)                  │    │
│  │ - mDNS 浏览 (发现局域网设备)                            │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

## 消息协议设计

### 帧格式（TCP 流）

```
┌──────┬────────────────┬──────────────────────┐
│ type │ length (BE u32)│ payload              │
│ 1B   │ 4B             │ variable             │
└──────┴────────────────┴──────────────────────┘
```

| type | 含义 | payload |
|:-----|:-----|:--------|
| `0x00` | 握手消息 | HandshakeMessage JSON |
| `0x01` | 数据消息 | SignalMessage JSON（现有协议复用） |
| `0x02` | 加密数据（预留） | ChaCha20-Poly1305 密文 |

### 握手消息（帧类型 0x00）

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum HandshakeMessage {
    #[serde(rename = "connect_request")]
    ConnectRequest {
        device_name: String,   // 发起方设备名
        peer_id: String,       // 发起方公钥指纹（展示用）
        public_key: String,    // 发起方 X25519 公钥（hex）
    },

    #[serde(rename = "auth_code")]
    AuthCode {
        code: String,          // 6 位数字配对码
    },

    #[serde(rename = "auth_result")]
    AuthResult {
        success: bool,
        peer_name: Option<String>,     // 响应方设备名
        public_key: Option<String>,    // 响应方 X25519 公钥（hex）
    },
}
```

### 首次连接流程

```
发起方 (Initiator)                          响应方 (Responder)
    │                                           │
    │── TCP connect ─────────────────────────→  │
    │── frame(0x00, ConnectRequest) ─────────→  │
    │                                           │ 检查 public_key 是否在 trusted_peers
    │                                           │ → 未找到，生成随机 6 位配对码
    │                                           │ → 弹出 UI: "配对码: 870921"
    │                                           │
    │   用户肉眼看到响应方屏幕上的 870921        │
    │                                           │
    │── frame(0x00, AuthCode{code:"870921"}) →  │
    │                                           │ 验证通过
    │                                           │ 保存发起方公钥到 trusted_peers
    │←─ frame(0x00, AuthResult{success:true}) ─│
    │                                           │
    │   保存响应方公钥到 trusted_peers           │
    │                                           │
    │═══════ 帧类型切换为 0x01 (数据) ══════════│
    │                                           │
    │── frame(0x01, SignalMessage::Clipboard) → │  ← 复用现有协议
    │←─ frame(0x01, SignalMessage::Clipboard) ──│
```

### 重连流程（已配对设备）

```
发起方                                  响应方
    │                                       │
    │── TCP connect ─────────────────────→  │
    │── frame(0x00, ConnectRequest) ─────→  │
    │                                       │ 检查 public_key → 在 trusted_peers 中!
    │                                       │ → 跳过配对码弹窗，直接接受
    │←─ frame(0x00, AuthResult{ok}) ───────│
    │                                       │
    │═══════ 帧类型切换为 0x01 ══════════════│
```

### 握手失败路径

```
# 配对码错误
发起方 ──AuthCode{code:"wrong"}──→ 响应方
发起方 ←──AuthResult{success:false}── 响应方  (TCP 关闭)

# 配对码过期 (60s)
发起方 ──AuthCode{code:"870921"}──→ 响应方
发起方 ←──AuthResult{success:false}── 响应方  (TCP 关闭，弹出已消失)

# 拒绝连接
发起方 ──ConnectRequest──→ 响应方 (用户点击"拒绝")
发起方 ←──AuthResult{success:false}── 响应方  (TCP 关闭)

# 未知设备连接（不在 trusted_peers 且用户拒绝查看）
响应方直接关闭 TCP（不回复任何帧）
```

## 数据结构变更

### AppConfig 扩展

```rust
// storage/json.rs

pub struct AppConfig {
    pub device_name: String,
    pub key_pair: Option<KeyPairData>,
    pub paired_peer: Option<PeerData>,       // 保留，向后兼容
    pub tcp_port: Option<u16>,                // 新增，默认 19876
    pub lan_enabled: Option<bool>,            // 新增，默认 true
    pub trusted_peers: Option<Vec<TrustedPeerData>>,  // 新增
}

pub struct TrustedPeerData {
    pub name: String,           // 设备名
    pub public_key: Vec<u8>,    // X25519 公钥 32 字节
    pub peer_id: String,        // 公钥指纹（展示/匹配用）
    pub last_ip: Option<String>, // 最近连接的 IP
}
```

### AppState 扩展

```rust
// lib.rs

pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub key_pair: Arc<Mutex<Option<KeyPair>>>,
    pub connected: Arc<Mutex<bool>>,
    pub connection: Arc<Mutex<Option<ConnectionHandle>>>,
    pub clip_tx: broadcast::Sender<ClipboardSnapshot>,

    // 新增
    pub lan_devices: Arc<Mutex<Vec<LanDevice>>>,           // mDNS 发现的设备列表
    pub pending_handshake: Arc<Mutex<Option<PendingHandshake>>>,  // 等待配对的入站连接
}
```

## 模块设计

### `network/discovery.rs` — mDNS 发现模块（新增）

```rust
// 职责：mDNS 服务注册 + 设备浏览
// 对外接口：

pub struct DiscoveryHandle {
    // 用于优雅关闭
}

impl DiscoveryHandle {
    /// 启动 mDNS：注册自己的服务 + 持续浏览局域网设备
    /// 发现新设备时通过 tx 通道通知上层
    pub async fn start(
        device_name: String,
        peer_id: String,
        port: u16,
        device_tx: mpsc::UnboundedSender<LanDeviceEvent>,
    ) -> Result<Self, Box<dyn Error>>;

    /// 停止 mDNS 并取消注册
    pub fn shutdown(&self);
}

pub struct LanDevice {
    pub name: String,
    pub peer_id: String,
    pub ip: String,
    pub port: u16,
}

pub enum LanDeviceEvent {
    Added(LanDevice),
    Removed(LanDevice),
}
```

- 使用 `mdns-sd` crate
- 服务类型: `_planarclip._tcp.local.`
- 注册时携带 TXT 记录: `peer_id`, `device_name`
- 浏览时解析 TXT 记录提取设备信息
- 设备变化通过 channel 推送给 lib.rs

### `network/direct.rs` — TCP 直连模块（新增）

```rust
// 职责：TCP 帧编解码 + 握手状态机
// 对外接口：

/// TCP 帧读取
pub async fn read_frame(reader: &mut TcpStream) -> Result<Frame, Error>;

/// TCP 帧写入
pub async fn write_frame(writer: &mut TcpStream, frame: &Frame) -> Result<(), Error>;

pub enum Frame {
    Handshake(HandshakeMessage),  // 帧类型 0x00
    Data(SignalMessage),          // 帧类型 0x01
}

/// 发起方：连接到远端并完成握手
/// 返回传输通道 + 对端信息
pub async fn connect_as_initiator(
    ip: &str,
    port: u16,
    device_name: &str,
    key_pair: &KeyPair,
    trusted_peers: &[TrustedPeerData],
) -> Result<DirectConnection, ConnectError>;

/// 响应方：接受入站连接并等待握手完成
/// 配对码通过 code_rx 返回给上层
pub async fn accept_as_responder(
    stream: TcpStream,
    device_name: &str,
    key_pair: &KeyPair,
    trusted_peers: &[TrustedPeerData],
    code_tx: oneshot::Sender<String>,       // 生成的配对码 → 前端展示
    reject_rx: oneshot::Receiver<()>,       // 前端点击"拒绝" → 中断
) -> Result<DirectConnection, AcceptError>;

/// 握手完成后的双向传输通道
/// 接口与现有 SignallingClient 对齐（rx/tx 相同签名）
pub struct DirectConnection {
    pub rx: mpsc::UnboundedReceiver<SignalMessage>,
    pub tx: mpsc::UnboundedSender<String>,      // JSON 字符串（与现有一致）
    pub peer_name: String,
    pub peer_id: String,
    pub peer_public_key: Vec<u8>,
}
```

**握手状态机（发起方）：**

```
Idle → TCP connect → SentRequest → 等待 AuthResult
                                   ← success: Connected
                                   ← failure: Failed

# 如果响应方需要配对码（非重连），前端会通过 Tauri command 调用 submit_pairing_code
# 前端调用时机：用户输入配对码后
```

**握手状态机（响应方）：**

```
Idle → 收到 TCP accept → ReceivedRequest
    → 公钥在 trusted_peers → 直接发送 AuthResult(success) → Connected
    → 公钥不在 → 生成配对码 → 发送事件给前端 → WaitingForCode
        → 收到 AuthCode 且正确 → 发送 AuthResult(success) → Connected
        → 收到 AuthCode 但错误 → 发送 AuthResult(failure) → Failed
        → 收到 reject 信号 → 发送 AuthResult(failure) → Failed
        → 60s 超时 → 发送 AuthResult(failure) → Failed
```

### `network/protocol.rs` — 新增 HandshakeMessage

```rust
// 现有内容不变，追加：

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HandshakeMessage {
    #[serde(rename = "connect_request")]
    ConnectRequest {
        device_name: String,
        peer_id: String,
        public_key: String,
    },
    #[serde(rename = "auth_code")]
    AuthCode { code: String },
    #[serde(rename = "auth_result")]
    AuthResult {
        success: bool,
        peer_name: Option<String>,
        public_key: Option<String>,
    },
}
```

### `lib.rs` — 集成变更

**新增 Tauri 命令：**

```rust
/// 获取当前发现的局域网设备列表
#[tauri::command]
async fn get_lan_devices(state: ...) -> Result<Vec<LanDevice>, String>;

/// 发起局域网连接（点击设备列表中的设备）
#[tauri::command]
async fn connect_lan(state: ..., ip: String, port: u16) -> Result<String, String>;

/// 提交配对码（发起方输入后）
#[tauri::command]
async fn submit_pairing_code(state: ..., code: String) -> Result<String, String>;

/// 拒绝连接申请（响应方点击"拒绝"）
#[tauri::command]
async fn reject_connection(state: ...) -> Result<(), String>;
```

**新增 Tauri 事件（推送给前端）：**

```rust
/// 设备列表变化
app_handle.emit("lan-devices-changed", &devices)?;

/// 收到连接申请（响应方弹出配对码弹窗）
app_handle.emit("connection-request", serde_json::json!({
    "device_name": "...",
    "peer_id": "...",
    "pairing_code": "870921",
}))?;

/// 连接已建立
app_handle.emit("connection-established", serde_json::json!({
    "peer_name": "...",
    "peer_id": "...",
    "is_reconnect": true/false,
}))?;

/// 连接失败/断开
app_handle.emit("connection-failed", reason)?;
```

**`run()` 函数启动流程变更：**

```
现有：load config → init state → setup tray → spawn monitor+engine
新流程：load config → init state → setup tray
        → spawn mDNS discovery       ← 新增
        → spawn TCP listener         ← 新增
        → spawn monitor+engine
```

TCP listener 在后台持续监听，每个入站连接 spawn 一个 handshake task。

### `network/webrtc.rs` — ConnectionManager 扩展

```rust
impl ConnectionManager {
    // 现有：通过信令服务器连接
    pub async fn connect(signalling_url, room, peer_id, ...) -> Result<ConnectionHandle>;

    // 新增：通过 TCP 直连
    pub async fn connect_direct(
        conn: DirectConnection,
        connected: Arc<Mutex<bool>>,
        clip_tx: broadcast::Sender<ClipboardSnapshot>,
    ) -> Result<ConnectionHandle>;
}
```

`connect_direct` 的内部逻辑与现有 `connect` 几乎相同——用 `conn.rx`/`conn.tx` 替代信令的 `sig_rx`/`sig_tx`，spawn 同样的 receive loop。

## 文件变更清单

| 操作 | 文件 | 说明 |
|:---|:---|:---|
| 新增 | `network/discovery.rs` | mDNS 注册 + 浏览 |
| 新增 | `network/direct.rs` | TCP 帧编解码 + 握手状态机 |
| 修改 | `network/mod.rs` | 添加 `pub mod discovery; pub mod direct;` |
| 修改 | `network/protocol.rs` | 追加 `HandshakeMessage` 枚举 |
| 修改 | `network/webrtc.rs` | `ConnectionManager::connect_direct()` |
| 修改 | `lib.rs` | AppState 扩展、新增命令、启动流程变更 |
| 修改 | `storage/json.rs` | AppConfig 扩展（tcp_port, trusted_peers） |
| 修改 | `Cargo.toml` | 添加 `mdns-sd` 依赖 |
| 修改 | `main.ts` | 前端 UI：设备列表、配对码弹窗 |

## 实施顺序

```
Phase 1: 基础设施（无前端可见变化）
├── 1.1 添加 mdns-sd 依赖
├── 1.2 AppConfig 扩展（tcp_port, trusted_peers）
├── 1.3 network/direct.rs — 帧编解码器（read_frame / write_frame）
└── 1.4 network/protocol.rs — HandshakeMessage 枚举

Phase 2: 连接流程（可在 Rust 层测试）
├── 2.1 network/direct.rs — 握手状态机 + TCP listener
├── 2.2 network/discovery.rs — mDNS 注册 + 浏览
├── 2.3 ConnectionManager::connect_direct()
└── 2.4 lib.rs — AppState 扩展 + 新 Tauri 命令

Phase 3: 前端（端到端可用）
├── 3.1 局域网设备列表 UI
├── 3.2 配对码弹窗（响应方）
├── 3.3 输入配对码（发起方）
└── 3.4 重连流程验证

Phase 4: 打磨
├── 4.1 已有信令服务器模式回归验证
├── 4.2 错误处理和用户提示
└── 4.3 配置文件中 trusted_peers 持久化验证
```

## 向后兼容说明

- `AppConfig.paired_peer` 字段保留不删，旧配置文件可正常加载
- 现有 `pair` 命令不变，信令服务器连接路径完全保留
- `ConnectionHandle` 接口不变，`SyncEngine` 无需修改
- 新增字段全部使用 `Option`，旧配置文件缺失时使用默认值
- 帧类型 `0x02` 预留用于后续加密升级，数据结构无需变更
