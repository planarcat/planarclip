# 单机连接测试与局域网直连方案 — 待执行方案

> 生成时间: 2026-06-04
> 基于讨论: [01-single-machine-connection-testing.md](01-single-machine-connection-testing.md) | [02-third-party-signalling-vs-direct-connection.md](02-third-party-signalling-vs-direct-connection.md) | [03-when-cross-network-is-needed.md](03-when-cross-network-is-needed.md) | [04-deprioritize-cross-network.md](04-deprioritize-cross-network.md)

## 需求概述

围绕 PlanarClip 的当前 MVP，先建立可重复的单机连接测试链路，并将后续连接方案聚焦到“无第三方云服务的局域网内通信”。当前阶段暂不考虑跨网能力，优先验证单机双实例测试、局域网设备发现与局域网内直连同步的可行性。

## 技术决策

| 决策项 | 选择 | 理由 | 来源轮次 |
|--------|------|------|----------|
| 单机测试方式 | 本地 WebSocket relay + 双实例 | 与当前实现最一致，验证成本最低 | 第 1 轮 |
| 实例隔离 | 不同 `APPDATA` | 避免两个实例共用身份与配置 | 第 1 轮 |
| 当前连接模型 | room join + relay 中继 | 现有 `pair(code)` 已按该模型实现 | 第 1 轮 |
| 是否必须第三方云服务器 | 否 | 同局域网下可用本地发现或直连替代 | 第 2 轮 |
| 无第三方可行路线 | 局域网自动发现 + 直连 | 是最现实、可用性最高的去第三方方案 | 第 2 轮 |
| 跨网需求认知 | 存在典型场景，但当前阶段不纳入范围 | 先收敛实现复杂度 | 第 3、4 轮 |
| 当前产品范围 | 单机测试 + 局域网直连 | 与当前目标和实现阶段最匹配 | 第 4 轮 |
| 暂不考虑内容 | signalling 公网部署 / STUN / TURN / NAT 穿透 | 现阶段不做跨网 | 第 4 轮 |

## 架构设计

### 方案 A：当前 MVP / 单机测试链路

```text
┌─────────────────────┐
│ Local WS Relay:8765 │
└─────────┬───────────┘
          │
   ┌──────┴──────┐
   │             │
┌──▼──┐       ┌──▼──┐
│ A   │       │ B   │
│APPDATA=A    │APPDATA=B
└─────┘       └─────┘
```

适用：
- 当前代码验证
- 单机双实例开发测试
- 验证配对、消息收发、剪贴板同步链路

### 方案 B：无第三方云服务的局域网直连

```text
┌──────────────┐      mDNS / UDP Broadcast      ┌──────────────┐
│ Device A     │ <────────────────────────────> │ Device B     │
│ announce/listen                              │ announce/listen
└──────┬───────┘                                └──────┬───────┘
       │                                                     │
       └──────────── TCP / UDP direct sync channel ──────────┘
```

适用：
- 同一局域网内的两台设备
- 无公网依赖
- 强调“装上即可在附近设备间发现并连接”

### 当前不纳入范围的方案

```text
Device A ── Signalling / STUN / TURN ── Device B
```

说明：
- 保留为未来扩展方向；
- 当前阶段不进入设计与实现。

## 范围边界

### 当前纳入范围
- 单机双实例连接测试
- 本地 relay 验证
- 局域网内设备发现
- 局域网内直连同步
- 基础配对与身份校验

### 当前不纳入范围
- 跨网络连接
- 公网信令服务器部署
- STUN / TURN
- NAT 穿透
- 异地设备重连与跨网状态同步

## 实现步骤

### Phase 1: 建立最小可行单机测试链路
- [ ] 准备本地 `ws://localhost:8765` relay
- [ ] 验证 relay 支持 `/room/{code}/peer/{peer_id}` 路径
- [ ] 使用不同 `APPDATA` 启动实例 A
- [ ] 使用不同 `APPDATA` 启动实例 B
- [ ] 在 A/B 中输入同一个 room code 并点击 Pair
- [ ] 验证两边 UI 状态都进入 `Connected`

### Phase 2: 验证文本消息传输
- [ ] 检查客户端日志是否出现连接成功与 peer join 事件
- [ ] 在 A 复制文本并验证 B 收到
- [ ] 在 B 复制文本并验证 A 收到
- [ ] 确认去重逻辑未造成回环同步

### Phase 3: 设计无第三方局域网连接模式
- [ ] 明确局域网模式是否优先采用自动发现还是手动输入地址
- [ ] 评估 mDNS / UDP 广播在 Windows/macOS 上的可行性
- [ ] 设计局域网设备发现 UI
- [ ] 设计局域网直连建立与身份校验流程
- [ ] 定义局域网内同步的数据通道方案

### Phase 4: 降低重复测试成本
- [ ] 为本地 relay 增加简单启动方式
- [ ] 整理双实例启动命令
- [ ] 视需要增加可配置数据目录或测试模式

## 关键依赖

### 当前 MVP / 测试链路
- Tauri v2 桌面客户端
- 本地 WebSocket relay（监听 `localhost:8765`）
- Windows `APPDATA` 环境变量隔离
- 当前已实现的 `pair(code)` 与剪贴板监控链路

### 无第三方局域网方案（候选）
- mDNS / UDP 广播发现机制
- 局域网 TCP / UDP 直连通道
- 设备身份与配对校验方案

## 推荐执行命令

在 `Apps/planarclip/` 目录下：

实例 A：

```powershell
$env:APPDATA="$env:TEMP\planarclip-a"; npx tauri dev
```

实例 B：

```powershell
$env:APPDATA="$env:TEMP\planarclip-b"; npx tauri dev
```

## 风险与注意事项

- 若未启动本地 relay，当前 MVP 配对一定失败。
- 若两个实例共用同一 `APPDATA`，测试结果将失真。
- 无第三方并不等于无发现机制；没有云服务器时，仍需局域网广播、手输地址或扫码交换连接信息。
- 局域网模式的可用性高度依赖网络环境是否允许设备互相发现与连接。
- 虽然当前不做跨网，但后续若定位变化，连接架构仍应保留扩展空间。

## 参考讨论

- [01-single-machine-connection-testing.md](01-single-machine-connection-testing.md)
- [02-third-party-signalling-vs-direct-connection.md](02-third-party-signalling-vs-direct-connection.md)
- [03-when-cross-network-is-needed.md](03-when-cross-network-is-needed.md)
- [04-deprioritize-cross-network.md](04-deprioritize-cross-network.md)
