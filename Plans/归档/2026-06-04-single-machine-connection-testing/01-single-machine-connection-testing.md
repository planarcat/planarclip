# 第 1 轮：这个项目我要先在单机状态下做连接测试，要怎么做？

> 时间: 2026-06-04 08:30

## 用户提问

这个项目我要先在单机状态下做连接测试，要怎么做？

## 背景与分析

当前仓库的连接测试实现具有两个关键前提。

1. 客户端默认连接本地信令服务：`Apps/planarclip/src-tauri/src/lib.rs` 中将 `SIGNALLING_SERVER` 固定为 `ws://localhost:8765`，因此没有本地 WebSocket relay 时，配对会直接失败。
2. 单机双开默认会共享同一份本地配置：`Apps/planarclip/src-tauri/src/storage/json.rs` 将配置固定存放在 `%APPDATA%/planarclip_config.json`。如果直接双开，两个实例会共享同一份密钥对与配置，导致它们不像两台独立设备。

进一步查看实现后，当前 MVP 的“连接测试”本质并不是 WebRTC P2P 测试，而是 WebSocket relay 路径上的联通性测试：

- 前端 `pair` 按钮会调用后端 `pair(code)` 命令。
- 后端 `pair` 会以 `room/{code}/peer/{peer_id}` 的路径连接信令服务。
- 当前传输通路主要依赖信令 WebSocket 中继文本消息，而不是完整的 ICE/STUN/TURN WebRTC 数据通道。

因此，最贴近现状的单机测试路径应为：

- 本机启动一个 `ws://localhost:8765` 的 WebSocket relay；
- 本机同时启动两个 Tauri 实例；
- 通过不同的 `APPDATA` 目录隔离两套配置；
- 让两个实例加入同一个 6 位房间号；
- 先验证连接状态，再验证文本剪贴板同步。

## 建议与回答

建议将“单机连接测试”拆成一个最小可行链路：**本地 relay + 双实例 + 配置隔离 + 同房间连接**。

### 一、准备本地信令服务

客户端当前会连接：

- `ws://localhost:8765/room/<6位配对码>/peer/<peer_id>`

所以需要一个本地 WebSocket 服务具备以下能力：

- 接受上述路径格式的连接；
- 按 `room` 分组管理连接；
- 将某个 peer 发来的文本消息转发给同一 room 的其他 peer；
- 可选广播 `peer_joined` / `peer_left`。

当前仓库中还没有现成服务端，因此单机测试前需要补一个本地 relay，或临时手写一个测试服务器。

### 二、双开实例时隔离配置

在 `Apps/planarclip/` 目录下分别启动两个实例，并为它们设置不同的 `APPDATA`：

终端 A：

```powershell
$env:APPDATA="$env:TEMP\planarclip-a"; npx tauri dev
```

终端 B：

```powershell
$env:APPDATA="$env:TEMP\planarclip-b"; npx tauri dev
```

这样两个实例会分别生成：

- `planarclip-a/planarclip_config.json`
- `planarclip-b/planarclip_config.json`

从而具备两套不同身份，可模拟两台独立设备。

### 三、连接测试步骤

1. 启动本地 relay。
2. 启动实例 A。
3. 启动实例 B。
4. 在两个实例中都输入同一个 6 位数字，例如 `123456`。
5. 两边都点击 Pair。
6. 观察两边状态是否变为 `Connected`。

### 四、同步测试步骤

连接成功后，再测文本同步：

1. 在 A 上复制一段文本，例如 `hello from A`；
2. 切到 B 上粘贴，验证是否收到；
3. 再在 B 上复制 `hello from B`；
4. 回到 A 上粘贴，验证反向同步。

### 五、推荐的测试顺序

为了提高排查效率，建议按以下顺序验证：

1. **先测能否连上**：只关注 `Connected` 状态；
2. **再测消息是否可达**：观察 relay 和客户端日志；
3. **最后测剪贴板同步**：验证双向文本复制粘贴。

### 六、最省事的后续落地方向

如果希望之后能反复稳定执行单机测试，建议补三样东西：

- 一个本地测试专用 signalling relay；
- 启动脚本或说明文档，明确三终端测试流程；
- 可配置的数据目录支持，避免每次手动改 `APPDATA`。

## 本轮要点

- 当前单机连接测试依赖本地 `ws://localhost:8765` relay。
- 当前 MVP 测的是 WebSocket relay 联通性，不是完整 WebRTC P2P。
- 双开实例必须使用不同 `APPDATA` 才能模拟双设备。
- 推荐测试顺序是：先连通，再收发，再剪贴板同步。
