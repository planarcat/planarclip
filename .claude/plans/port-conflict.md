# 端口冲突处理(块 C 实施细节)

## 目标
应用启动时端口被占,**不杀任何进程**,提示用户改端口;不改则不广播;改到可用端口后**自动开始广播**;新端口仍不可用则继续提示。

## 后端

### C1 端口可用性检测
- `direct.rs`: `is_port_available(port: u16) -> bool`——`std::net::TcpListener::bind(("0.0.0.0", port))` 测试,成功即 drop 返回 true。

### C2 BroadcastState + AppState
- `BroadcastState` enum(`serde::Serialize`): `Active { port }` / `PortConflict { port }` / `Inactive { reason }`。
- `AppState` 加两个字段:
  - `broadcast_state: Arc<Mutex<BroadcastState>>`
  - `broadcast_handles: Arc<Mutex<Option<BroadcastHandles>>>`
- `BroadcastHandles { discovery_daemon: ServiceDaemon, discovery_task, presence_task, listener_task, listener_event_task: JoinHandle<()> }`(auto_connect 一次性,不存 handle)。

### C3 start_broadcast / stop_broadcast 抽取
- `start_broadcast(app, port) -> Result<BroadcastHandles, String>`:封装现 setup 闭包里的 discovery 启动 + discovery 事件 loop + presence refresh loop + auto_connect(一次性)+ listener + listener 事件 loop([lib.rs:3104-3313](Apps/src-tauri/src/lib.rs:3104))。返回 handles。
- `stop_broadcast(handles)`:discovery_daemon.shutdown()(mdns_sd)+ abort 各 task JoinHandle。
- setup 启动时:`is_port_available(tcp_port)`。可用 -> `start_broadcast` -> `Active`。冲突 -> 不启动 -> `PortConflict` + `emit("broadcast-state-changed")`。

### C4 IPC
- `set_tcp_port(port)`:校验(1024–65535,禁 1420/1421) + 写 config + `is_port_available`:
  - 可用:stop 旧 handles(若 Some)+ `start_broadcast(新)` -> `Active` + emit。
  - 不可用:`PortConflict` + emit(不启动广播)。
- `get_broadcast_state()`:返回当前 `BroadcastState`。

## 前端

### C5
- `ShellBootstrapPayload` 加 `broadcast_state` 字段(启动初始状态)。
- 新事件 `broadcast-state-changed`;`useConnectionBridge` listen(参考 `lan-devices-changed`)。
- `SettingsPage` "连接"分区:加"监听端口"数字输入 + 保存(draft+commit,参考 maxFileMb)。
- 端口冲突模态(overlays):`PortConflict` 时弹模态"端口 X 被占用,请在设置中修改" + "去设置"按钮;改好可用后自动消失。

## 风险与注意
- 生命周期重构:discovery/listener/presence/auto_connect 都依赖 `tcp_port` 且各自 spawn,封装成 `start_broadcast`/`stop_broadcast` 是本块主要工作。
- 改端口时活跃连接会断(stop listener);若有连接,提示或优雅关闭。
- mdns_sd `ServiceDaemon` 的 shutdown/stop API 实施时确认(drop 一般不停止后台线程,需显式 shutdown)。
- `is_port_available` 检测后到实际 bind 之间的小竞态:bind 失败回退 `PortConflict`。

## 实施顺序
1. C1 + C2(基础:检测函数 + BroadcastState/AppState 字段)
2. C3(`start_broadcast`/`stop_broadcast` 抽取 + 启动检测)
3. C4(`set_tcp_port` / `get_broadcast_state` IPC)
4. C5(前端 bootstrap 字段 + 事件 + 设置页端口 + 冲突模态)

## 需跑 impact 的符号
`run`, setup 闭包, `start_discovery`, `run_listener`, `AppState`(新字段)
