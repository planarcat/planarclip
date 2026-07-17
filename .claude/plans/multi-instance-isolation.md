# 多实例隔离与端口冲突处理

## 目标
让同一台设备能同时启动多个互不冲突的 PlanarClip 实例(dev/prod 或同 profile 多副本),每个实例有独立身份、独立数据目录、独立端口;端口冲突时不杀任何进程,由用户改端口解决。

## 已确认决策
- 数据迁移:自动迁移老数据到新目录结构
- 数据根目录名:`PlanarClip`
- 端口冲突提示:启动模态 + 设置页可改
- `--instance` 启动参数:本轮暂缓(第 4 项,后续做)

## 前提认知(不重造身份层)
身份锚点 `peer_id` 已存在:基于持久化密钥对,首次生成([lib.rs:2400](Apps/src-tauri/src/lib.rs:2400) `load_or_create_key_pair`)、应用不主动改、用户可手改配置;全链路身份判定已基于 `peer_id` 而非 ip([lib.rs:937](Apps/src-tauri/src/lib.rs:937)、[auto_connect.rs:74](Apps/src-tauri/src/auto_connect.rs:74)、[discovery.rs:77](Apps/src-tauri/src/network/discovery.rs:77))。本方案只补齐三处:多实例数据隔离、mDNS 实例名、端口冲突处理。

## 目录结构(新)
```
Windows:  %APPDATA%\PlanarClip\<peer_id>\
macOS:    ~/Library/Application Support/PlanarClip\<peer_id>\
Linux:    ~/.config/PlanarClip\<peer_id>\
  ├─ config.json
  ├─ logs\planarclip.log   (dev: planarclip-dev.log)
  ├─ staging\
  └─ history_thumbs\
```
所有数据归入实例目录,多实例天然隔离。

---

## 块 A:数据目录重构 + 自动迁移

### A1. 实例路径模块
- 新增 `storage/paths.rs`(或扩展 `app_profile.rs`):
  - `data_root() -> PathBuf`:平台数据根 + `PlanarClip`
  - `InstancePaths { dir, config_path, log_dir, staging_root, thumbs_root }`
  - 启动时确定一次,存入 `OnceLock<InstancePaths>`,全局可取
- `config_path()/log_dir()/staging_root()/thumbs_root()` 改为从 `InstancePaths` 派生,保持现有调用点签名兼容

### A2. 实例定位(启动时,在 `logging::init` 之前)
- 扫描 `data_root()` 子目录(目录名 = peer_id)
- 0 个 + 老配置存在 -> 触发迁移(A3)
- 0 个 + 无老配置 -> 生成新 key_pair -> peer_id -> 创建实例目录
- 1 个 -> 用它
- 多个 -> 选最近修改的(日志提示;`--instance` 参数留待第 4 项)
- 确定后 `logging::init()`(此时 log_dir 指向实例目录)

### A3. 自动迁移
- 触发:`data_root` 无实例目录 + 老配置存在(release:`planarclip_config.json` / dev:`planarclip_config.dev.json`)
- 步骤:读老配置 -> key_pair -> peer_id -> 创建实例目录 -> 写 `config.json` -> 复制 `history_thumbs`(与 clipboard_history 关联的缩略图)
- 不迁:`staging`(临时)、`logs`(重建)
- 老文件保留作备份(不删)
- dev/release 各自迁移(读各自老配置文件名)

### A4. 适配调用点
- `config_path/load_config/save_config`、`log_dir`、`staging_root`、`thumbs_root` 全部跟随 `InstancePaths`
- 因 logs/staging/thumbs 已从 `config_path().parent()` 派生([logging.rs:71](Apps/src-tauri/src/logging.rs:71)、[staging.rs:11](Apps/src-tauri/src/storage/staging.rs:11)、[history_thumbs.rs:9](Apps/src-tauri/src/storage/history_thumbs.rs:9)),改 `config_path` 即自动归位

---

## 块 B:mDNS 实例名改用 peer_id

- [discovery.rs:42](Apps/src-tauri/src/network/discovery.rs:42) `ServiceInfo::new` 的实例名参数:`device_name` -> `peer_id`
- `device_name` 仍作为 TXT 记录显示名(已存在),UI 显示不变
- `mdns_service_fullname_prefix`([app_profile.rs:43](Apps/src-tauri/src/app_profile.rs:43))改用 peer_id;检查调用处 [lib.rs:230](Apps/src-tauri/src/lib.rs:230)
- 效果:同机多实例(不同 peer_id)mDNS 实例名不再冲突

---

## 块 C:端口冲突处理(不杀任何进程)

### C1. 端口可用性检测
- 新增 `network::port::is_port_available(port) -> bool`:尝试 `TcpListener::bind("0.0.0.0:port")`,成功立即 drop

### C2. 广播状态
- `AppState` 加 `broadcast_state: Arc<Mutex<BroadcastState>>`
- `enum BroadcastState { Active{port}, PortConflict{port}, Inactive{reason} }`

### C3. 启动流程改造
- setup 内,`start_discovery`/`run_listener` 前检测 tcp_port:
  - 可用 -> 启动广播(Active)
  - 不可用 -> 不启动(PortConflict),emit 事件通知前端
- 抽取 `start_broadcast(port)` / `stop_broadcast()`:持有 discovery daemon + listener 句柄,支持延迟启动与重启
  - 重构当前一次性 spawn 逻辑([lib.rs:2920](Apps/src-tauri/src/lib.rs:2920) discovery daemon sleep 保活、[direct.rs:987](Apps/src-tauri/src/network/direct.rs:987) listener loop accept)

### C4. 端口设置 command
- `set_tcp_port(port)`:校验(1024–65535,禁 1420/1421) -> 写 config -> 检测新端口:
  - 可用 -> stop 旧广播 + start 新端口广播 -> Active -> emit
  - 不可用 -> PortConflict,不广播 -> emit(前端提示)
- `get_broadcast_state()`:返回当前状态 + 端口

### C5. 前端
- `ShellBootstrapPayload` 加 `broadcast_state` 字段(初始状态)
- 新事件 `broadcast-state-changed` 推送变更;`useConnectionBridge` listen(参考现有 `lan-devices-changed` 模式)
- `SettingsPage` "连接"分区加"监听端口"字段(数字输入 + 保存,draft+commit 模式,参考 maxFileMb)
- 端口冲突模态(overlays):PortConflict 时弹模态"端口 X 被占用,请在设置中修改" + "去设置"按钮;改好可用后自动消失
- 不广播时:设备列表空,顶部显示"未广播"

---

## 块 D(暂缓)
- `--instance <id>` 启动参数:精确指定实例目录。实例定位已预留多目录选择(最近修改),参数后续加。

---

## 实施顺序(建议分两步)
1. **块 A + 块 B**:数据目录重构 + 迁移 + mDNS 实例名。数据层,独立可测,先落地。
2. **块 C**:端口冲突处理。依赖 A 的配置路径,但可独立于多实例。

## 风险与注意
- `OnceLock` 测试污染:`config_path` 改运行时确定,单测需可注入实例路径
- 迁移 `history_thumbs` 归属:dev/release 老共享一份,迁移时按各自 `clipboard_history` 关联复制
- 重启广播断活跃连接:改端口时若有活跃连接,需优雅关闭或提示
- mDNS 实例名变更:升级后旧名(device_name)超时移除、新名(peer_id)注册,短暂可能双重,可接受
- 端口检测竞态:检测可用后到 bind 之间被占(小概率),bind 失败回退 PortConflict

## 需跑 impact 的符号(按 CLAUDE.md)
`run`, setup 闭包, `start_discovery`, `run_listener`, `config_path`, `load_config`, `save_config`, `log_dir`, `staging_root`, `thumbs_root`, `load_or_create_key_pair`, `mdns_service_fullname_prefix`
