---
round: 7
topic: 可预见的开发难题与风险
date: 2026-05-26
---

# 第 7 轮 · 可预见的开发难题与风险

> **背景**：方案趋于完整，在进入开发前识别技术难点，评估风险。

---

## 用户提问（原文）

> 在具体开发中，有没有哪些可预见的难题

---

## 难题一览

| # | 难题 | 严重程度 | 阶段 | 可控性 |
|:---|:---|:---|:---|:---|
| 1 | WebRTC NAT 穿透 | 🔴 高 | Phase 1–3 | 部分可控 |
| 2 | 跨平台剪贴板格式差异 | 🟡 中 | Phase 1–2 | 可控 |
| 3 | WebRTC 大文件分块传输 | 🟡 中 | Phase 2 | 可控 |
| 4 | 自写入回环竞态 | 🟡 中 | Phase 1 | 可控 |
| 5 | Noise 加密与 WebRTC 集成 | 🟡 中 | Phase 1 | 可控 |
| 6 | arboard 跨平台兼容性 | 🟢 低 | Phase 1 | 可控 |
| 7 | 多对等点连接数爆炸 | 🟢 低 | Phase 3 | 可控 |
| 8 | macOS 沙盒授权 | 🟢 低 | Phase 4 | 可控 |

---

## 1. 🔴 WebRTC NAT 穿透（最高风险）

**问题**：P2P 直连的前提是两个设备能建立 WebRTC 连接。

- STUN 服务器仅能穿透 ~85% 的 NAT 类型
- 对称 NAT（企业网络常见）必须依赖 TURN 中继
- TURN 中继有带宽成本，且数据经过第三方，与隐私目标矛盾
- `webrtc-rs` 生态不如 Chromium 的 WebRTC 成熟

**缓解策略**：
- Phase 1 先支持同局域网（mDNS + STUN），Cover 80% 场景
- Phase 3 加入 TURN 回退，提供默认 TURN 服务器
- 探测失败时降级为通过信令服务器中转（小数据），提示用户"直连失败，已使用中继模式"

---

## 2. 🟡 跨平台剪贴板格式差异

**问题**：Windows 和 macOS 的剪贴板格式体系完全不同。

| 场景 | Windows | macOS |
|:---|:---|:---|
| 文本 | `CF_UNICODETEXT` | `NSPasteboardTypeString` |
| 图片 | `CF_DIB` / `CF_BITMAP` | `NSPasteboardTypePNG` / `NSPasteboardTypeTIFF` |
| 文件列表 | `CF_HDROP` (DROPFILES 结构) | `NSFilenamesPboardType` (已废弃，需用 `NSURL` ) |
| HTML | `CF_HTML` | `NSPasteboardTypeHTML` |

`arboard` 对文本和图片支持良好，**文件列表是薄弱点**。`arboard` 目前不直接支持获取文件列表，需要平台特定代码。

**缓解策略**：
- Phase 1 仅处理文本（arboard 完全支持）
- Phase 2 图片用 arboard::get_image()
- Phase 2 文件列表：Win 用 `windows-sys` 读 `CF_HDROP`，Mac 用 `objc2` 读 `NSPasteboard`
- 维护一个 `ClipboardAdapter` trait，平台各自实现

---

## 3. 🟡 WebRTC 大文件分块传输

**问题**：WebRTC Data Channel 基于 SCTP，单条消息有 ~16KB 限制（部分实现更小）。大文件需要分块。

- 需自行实现分块协议（chunk index / total / hash / ack / retransmit）
- 流控：发送太快会填满缓冲区导致丢包
- 内存：不能把 2GB 文件全读入内存
- 多文件同时传输时的优先级调度

**缓解策略**：
- 参考已有方案（FileXile、Snapdrop 的分块策略）
- Phase 2 实现基础分块，单文件串行传输
- 使用内存映射（mmap）逐块读取大文件
- 每块带 blake3 哈希，接收端逐块验证
- 流控：发送窗口 = 8 块，收到 ACK 后滑动

---

## 4. 🟡 自写入回环竞态

**问题**：同步引擎写入剪贴板 → 本地监控器检测到变化 → 再次广播 → 无限循环。

当前方案用 `AtomicBool` 标志位，但存在窗口：
1. 线程 A 设置 `self_writing = true`
2. 线程 A 调用 `clipboard.set_text()`
3. **此时 poll 线程刚读完标志位（读到 false），错过窗口**
4. poll 线程读到剪贴板（已是新内容）→ 触发同步

**缓解策略**：
- 写入前设置标志 → 写入 → 读取确认（hash 对比）→ 清除标志
- 写入后立即更新 `last_hash`，使 poll 线程下一轮认为"无变化"
- 更稳健的方案：为远程写入的内容打上水印（如剪贴板数据中嵌入不可见标记，但不通用）

实际上 `AtomicBool` + 写入后更新 `last_hash` 的组合足以应对 500ms 轮询间隔下的绝大多数情况。

---

## 5. 🟡 Noise 加密与 WebRTC 数据通道集成

**问题**：`snow` (Noise) 和 `webrtc-rs` 是两个独立 crate，需正确串联。

- Noise IK 握手消息需要通过 WebRTC Data Channel 传输
- 握手期间数据通道上传输的不是"业务消息"而是"握手帧"
- 握手完成后切换为传输模式
- WebRTC 连接断开后需重新握手（新的 Noise session）

**缓解策略**：
- 在 Data Channel 上定义一个简单的帧类型前缀（0x00 = Noise 握手帧，0x01 = 加密业务帧）
- Noise 握手完成后立即派生 TransportState，切换模式
- 参考 TLS over TCP 的模式设计状态机

---

## 6. 🟢 arboard 跨平台兼容性

**问题**：`arboard` 在不同平台上的行为有细微差异。

- Linux 上依赖 X11/Wayland，行为可能不稳定
- macOS 上某些图片格式可能转换失败
- Windows 上 UWP 应用写入的剪贴板内容可能读不到

**缓解策略**：
- 本项目仅支持 Win + Mac，Linux 不在范围内，规避了最大风险
- Phase 1 仅文本，arboard 在 Win/Mac 上文本支持稳定
- 图片和文件列表在 Phase 2 通过平台特定代码补充

---

## 7. 🟢 多对等点连接数爆炸

**问题**：全网格（full mesh）下 N 台设备需要 N×(N-1)/2 条 WebRTC 连接。5 台设备 = 10 条连接，每增加一台线性开销增长。

**缓解策略**：
- Phase 3 先支持全网格（实际使用场景设备数大概率 ≤ 5）
- 未来可选升级为星型拓扑：选一台设备做信号转发（类似局域网内的"小信令"）
- 动态评估：设备数 > 5 时弹出提示"设备较多，建议指定一台设备作为中继"

---

## 8. 🟢 macOS 沙盒与授权

**问题**：macOS 对剪贴板访问有沙盒限制。

- App Store 分发需要启用沙盒，可能限制剪贴板访问
- 系统托盘（NSStatusBar）需要特定 entitlement
- 自启动（Login Item）需要额外配置
- 公证（Notarization）需要 Apple Developer 账号

**缓解策略**：
- Phase 1–3 非 App Store 分发，通过 DMG + 公证即可
- Tauri 2 已封装大部分 macOS 权限配置
- 发布阶段再处理 App Store 兼容

---

## 📊 风险矩阵

```
                    可能性
              低          中          高
          ┌──────────┬──────────┬──────────┐
严   高   │          │   NAT穿透  │          │
重        │          │   (TURN回退)│          │
性   中   │          │ 剪贴板格式  │          │
          │          │ 大文件分块  │          │
          │          │ 回环竞态    │          │
          │          │ Noise集成  │          │
     低   │ arboard  │          │          │
          │ 多对等点  │          │          │
          │ macOS沙盒 │          │          │
          └──────────┴──────────┴──────────┘
```

---

## 📌 本轮要点

- **最高风险：NAT 穿透** — STUN 不够时降级 TURN / 信令中继
- **文件列表**是 arboard 的盲区，需平台特定代码（CF_HDROP / NSPasteboard）
- **大文件分块**需自建协议，参考 Snapdrop 等成熟方案
- **回环竞态**用 `AtomicBool` + 写入后更新 `last_hash` 可解决
- 其余问题（Noise 集成、多对等点、macOS 沙盒）各有成熟缓解方案，风险可控
- 没有不可逾越的障碍，所有难题都有已知解法

---

*[← 上一轮](06-pending-decisions-resolved.md)*
