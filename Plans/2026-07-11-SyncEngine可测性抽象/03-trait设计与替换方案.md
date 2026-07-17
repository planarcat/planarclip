# 第 3 轮：trait 设计与替换方案

> 时间: 2026-07-11

## 用户提问

（内部推进，接续第 2 轮）影响面已经收敛到 4 个方法 + 1 个上游构造点。本轮把 trait 长什么样、生产/测试各自怎么实现、`ConnectionRegistry` 那层怎么塞入抽象一次性设计清楚，落地时照抄即可。

## 背景与分析

### 抽象边界回顾

第 2 轮结论：外部世界对 `SyncEngine` 有效的行为只有 4 个 —— 3 个 `send_*` + 1 个 `supports_chunked_images`。所有 `AppHandle` 的使用都在 `broadcast_snapshot` 分派后进入 `ConnectionHandle::send_*`；一旦这 4 个方法进入 trait，`AppHandle` 也顺势内收进生产实现，不再出现在 `SyncEngine` 本体上。

### 双 trait 分工

SyncEngine 与外部有两处耦合：

1. **调用面**（把快照推送出去）→ `ClipboardOut` trait
2. **发现面**（当前有哪些活跃连接）→ `ClipboardOutProvider` trait

两个 trait 一起换掉后，`SyncEngine` 完全脱离 `ConnectionHandle` / `ConnectionRegistry` / `AppHandle`。

```
SyncEngine { rx, provider, transfer_slots, config }
      │
      │  provider.active_outs().await
      ▼
Vec<Arc<dyn ClipboardOut>>
    ├─ supports_chunked_images()
    ├─ send_snapshot()
    ├─ send_image_async()
    └─ send_files_async()
```


## trait 定义

新增 `Apps/src-tauri/src/sync/out.rs`；`sync/mod.rs` 加 `pub mod out;`。

```rust
// Apps/src-tauri/src/sync/out.rs
use std::sync::Arc;
use async_trait::async_trait;
use crate::clipboard::types::ClipboardSnapshot;

#[async_trait]
pub trait ClipboardOut: Send + Sync {
    fn supports_chunked_images(&self) -> bool;

    fn send_snapshot(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool);

    async fn send_image_async(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool);

    async fn send_files_async(
        &self,
        snapshot: Arc<ClipboardSnapshot>,
        sync_files: bool,
        max_file_bytes: u64,
    );
}

#[async_trait]
pub trait ClipboardOutProvider: Send + Sync {
    async fn active_outs(&self) -> Vec<Arc<dyn ClipboardOut>>;
}
```

> 依赖增补：`Cargo.toml [dependencies]` 增 `async-trait = "0.1"`。与 `dyn Trait + Send` 组合最稳；这是本方案唯一新增的运行时依赖。

### 参数形态相对现状的两处调整

1. **拿掉 `Option<AppHandle>` / `Option<&AppHandle>`**：生产实现自持 `AppHandle`；测试实现不需要。
2. **快照统一为 `Arc<ClipboardSnapshot>`**：`send_snapshot` 当前是 `&ClipboardSnapshot`（同步），两个 `send_*_async` 当前是 `ClipboardSnapshot` 按值（异步）。统一为 `Arc<_>` 后 4 个分支共用一份克隆，`.await` 边界 `Send` 稳定。

`SyncEngine::broadcast_snapshot` 内部把 `event.snapshot: ClipboardSnapshot` 一次 `Arc::new(event.snapshot)` 后共享给所有 outs。


## 生产实现：`TauriConnectionOut` + `RegistryOutProvider`

```rust
use tauri::AppHandle;
use tokio::sync::Mutex;
use crate::network::sessions::ConnectionRegistry;
use crate::network::webrtc::ConnectionHandle;

pub struct TauriConnectionOut {
    inner: ConnectionHandle,
    app: AppHandle,
}

impl TauriConnectionOut {
    pub fn new(inner: ConnectionHandle, app: AppHandle) -> Self {
        Self { inner, app }
    }
}

#[async_trait]
impl ClipboardOut for TauriConnectionOut {
    fn supports_chunked_images(&self) -> bool {
        self.inner.supports_chunked_images()
    }

    fn send_snapshot(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool) {
        self.inner
            .send_snapshot(snapshot.as_ref(), sync_images, Some(&self.app));
    }

    async fn send_image_async(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool) {
        self.inner
            .send_image_async((*snapshot).clone(), sync_images, Some(self.app.clone()))
            .await;
    }

    async fn send_files_async(
        &self,
        snapshot: Arc<ClipboardSnapshot>,
        sync_files: bool,
        max_file_bytes: u64,
    ) {
        self.inner
            .send_files_async(
                (*snapshot).clone(),
                sync_files,
                max_file_bytes,
                Some(self.app.clone()),
            )
            .await;
    }
}

pub struct RegistryOutProvider {
    registry: Arc<Mutex<ConnectionRegistry>>,
    app: AppHandle,
}

impl RegistryOutProvider {
    pub fn new(registry: Arc<Mutex<ConnectionRegistry>>, app: AppHandle) -> Self {
        Self { registry, app }
    }
}

#[async_trait]
impl ClipboardOutProvider for RegistryOutProvider {
    async fn active_outs(&self) -> Vec<Arc<dyn ClipboardOut>> {
        let registry = self.registry.lock().await;
        registry
            .active_handles()
            .into_iter()
            .map(|handle| {
                Arc::new(TauriConnectionOut::new(handle, self.app.clone()))
                    as Arc<dyn ClipboardOut>
            })
            .collect()
    }
}
```


## 测试实现：`InMemoryOut` + `StaticOutsProvider`

```rust
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::sync::Mutex as StdMutex;

    #[derive(Debug, Clone, PartialEq)]
    pub enum RecordedCall {
        Snapshot   { sync_images: bool,  snapshot: ClipboardSnapshot },
        ImageAsync { sync_images: bool,  snapshot: ClipboardSnapshot },
        FilesAsync { sync_files: bool, max_file_bytes: u64,
                     snapshot: ClipboardSnapshot },
    }

    pub struct InMemoryOut {
        pub supports_chunked: bool,
        pub calls: Arc<StdMutex<Vec<RecordedCall>>>,
    }

    impl InMemoryOut {
        pub fn new(supports_chunked: bool) -> Arc<Self> {
            Arc::new(Self {
                supports_chunked,
                calls: Arc::new(StdMutex::new(Vec::new())),
            })
        }
        pub fn recorded(&self) -> Vec<RecordedCall> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ClipboardOut for InMemoryOut {
        fn supports_chunked_images(&self) -> bool { self.supports_chunked }
        fn send_snapshot(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool) {
            self.calls.lock().unwrap().push(RecordedCall::Snapshot {
                sync_images, snapshot: (*snapshot).clone(),
            });
        }
        async fn send_image_async(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool) {
            self.calls.lock().unwrap().push(RecordedCall::ImageAsync {
                sync_images, snapshot: (*snapshot).clone(),
            });
        }
        async fn send_files_async(
            &self,
            snapshot: Arc<ClipboardSnapshot>,
            sync_files: bool,
            max_file_bytes: u64,
        ) {
            self.calls.lock().unwrap().push(RecordedCall::FilesAsync {
                sync_files, max_file_bytes, snapshot: (*snapshot).clone(),
            });
        }
    }

    pub struct StaticOutsProvider {
        pub outs: Vec<Arc<dyn ClipboardOut>>,
    }

    #[async_trait]
    impl ClipboardOutProvider for StaticOutsProvider {
        async fn active_outs(&self) -> Vec<Arc<dyn ClipboardOut>> {
            self.outs.clone()
        }
    }
}
```


## SyncEngine 新签名 & broadcast_snapshot 改写

```rust
// Apps/src-tauri/src/sync/engine.rs
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::clipboard::file::DEFAULT_MAX_FILE_BYTES;
use crate::clipboard::image::INLINE_IMAGE_BYTES;
use crate::clipboard::types::{ClipboardEvent, ClipboardOrigin, ClipboardSnapshot};
use crate::storage::json::AppConfig;
use crate::sync::out::{ClipboardOut, ClipboardOutProvider};
use crate::sync::transfer_limit::TransferSlotLimiter;

pub struct SyncEngine {
    rx: broadcast::Receiver<ClipboardEvent>,
    provider: Arc<dyn ClipboardOutProvider>,
    transfer_slots: Arc<TransferSlotLimiter>,
    config: Arc<Mutex<AppConfig>>,
}

impl SyncEngine {
    pub fn new(
        rx: broadcast::Receiver<ClipboardEvent>,
        provider: Arc<dyn ClipboardOutProvider>,
        transfer_slots: Arc<TransferSlotLimiter>,
        config: Arc<Mutex<AppConfig>>,
    ) -> Self {
        Self { rx, provider, transfer_slots, config }
    }

    pub async fn run(mut self) {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if matches!(event.origin, ClipboardOrigin::Remote { .. }) {
                        continue;
                    }
                    let (sync_images, sync_files, max_file_bytes, auto_sync_clipboard) = {
                        let cfg = self.config.lock().await;
                        (
                            cfg.sync_images.unwrap_or(true),
                            cfg.sync_files.unwrap_or(true),
                            cfg.max_file_bytes.unwrap_or(DEFAULT_MAX_FILE_BYTES),
                            cfg.auto_sync_clipboard.unwrap_or(true),
                        )
                    };
                    if !auto_sync_clipboard && !event.skip_history_merge {
                        continue;
                    }
                    let outs = self.provider.active_outs().await;
                    if outs.is_empty() { continue; }

                    let snapshot = Arc::new(event.snapshot);
                    Self::broadcast_snapshot(
                        outs,
                        snapshot,
                        sync_images,
                        sync_files,
                        max_file_bytes,
                        self.transfer_slots.clone(),
                    );
                }
                Err(e) => {
                    tracing::error!("Sync engine channel error: {}", e);
                    break;
                }
            }
        }
    }

    fn broadcast_snapshot(
        outs: Vec<Arc<dyn ClipboardOut>>,
        snapshot: Arc<ClipboardSnapshot>,
        sync_images: bool,
        sync_files: bool,
        max_file_bytes: u64,
        transfer_slots: Arc<TransferSlotLimiter>,
    ) {
        for out in outs {
            match snapshot.as_ref() {
                ClipboardSnapshot::Image { png_bytes, .. }
                    if png_bytes.len() > INLINE_IMAGE_BYTES
                        && out.supports_chunked_images() =>
                {
                    let out = out.clone();
                    let snapshot = snapshot.clone();
                    let slots = transfer_slots.clone();
                    tokio::spawn(async move {
                        let _permit = slots.acquire().await;
                        out.send_image_async(snapshot, sync_images).await;
                    });
                }
                ClipboardSnapshot::FileList { .. } if sync_files => {
                    let out = out.clone();
                    let snapshot = snapshot.clone();
                    let slots = transfer_slots.clone();
                    tokio::spawn(async move {
                        let _permit = slots.acquire().await;
                        out.send_files_async(snapshot, true, max_file_bytes).await;
                    });
                }
                _ => {
                    out.send_snapshot(snapshot.clone(), sync_images);
                }
            }
        }
    }
}
```

**4 分支等价性核查**（HIGH 点）：

| 快照类型 | 现状分支条件 | 新代码分支条件 | 是否行为等价 |
|:---|:---|:---|:---|
| 大 PNG 且支持分块 | `Image && len > INLINE && handle.supports_chunked_images()` | 完全一致 | ✅ |
| 文件列表且开启文件同步 | `FileList && sync_files` | 完全一致 | ✅ |
| 大 PNG 但不支持分块 | 落 `_` → `send_snapshot`（会走 webrtc 内 512KB 兜底提示） | 一致 | ✅ |
| 文本 / 内联小图 / 空 / 文件同步关闭 | 落 `_` → `send_snapshot` | 一致 | ✅ |


## `lib.rs:3219` 构造点替换

```rust
// 现状（约 3215–3220）：
// let engine = sync::engine::SyncEngine::new(
//     clip_rx, connections_bg, transfer_slots_bg, config_bg, app_handle_bg,
// );

// 改为：
use crate::sync::out::RegistryOutProvider;

let provider = std::sync::Arc::new(RegistryOutProvider::new(
    connections_bg.clone(),
    app_handle_bg.clone(),
));
let engine = sync::engine::SyncEngine::new(
    clip_rx,
    provider,
    transfer_slots_bg,
    config_bg,
);
```

改动量：新增一行 `use`、一段 3 行 `Arc::new(...)`、`SyncEngine::new` 少一个 `app_handle_bg` 入参、多一个 `provider` 入参。

## 测试怎么写（示意，Phase 3 实施）

放在 `Apps/src-tauri/src/sync/engine.rs` 底部 `#[cfg(test)] mod tests`：

```rust
#[tokio::test]
async fn text_snapshot_goes_to_send_snapshot_all_peers() {
    use crate::sync::out::test_support::{InMemoryOut, StaticOutsProvider, RecordedCall};

    let out_a = InMemoryOut::new(true);
    let out_b = InMemoryOut::new(false);
    let provider = Arc::new(StaticOutsProvider {
        outs: vec![out_a.clone(), out_b.clone()],
    });

    let (tx, rx) = broadcast::channel(4);
    let cfg = Arc::new(Mutex::new(AppConfig::default()));
    let slots = Arc::new(TransferSlotLimiter::new(2));
    let engine = SyncEngine::new(rx, provider, slots, cfg);
    tokio::spawn(engine.run());

    tx.send(ClipboardEvent {
        origin: ClipboardOrigin::Local,
        snapshot: ClipboardSnapshot::Text("hello".into()),
        skip_history_merge: false,
    }).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    for out in [&out_a, &out_b] {
        assert!(matches!(
            out.recorded().as_slice(),
            [RecordedCall::Snapshot { sync_images: true, snapshot: ClipboardSnapshot::Text(t) }] if t == "hello"
        ));
    }
}
```

**测试要覆盖的 4 个分支 + 2 个短路**（详细清单进 execution-plan Phase 3）：

1. Remote 来源事件 → 一个 out 也不调用
2. `auto_sync_clipboard=false` 且非 skip_history_merge → 一个 out 也不调用
3. 文本 → 所有 outs 收到 `Snapshot`
4. 大 PNG + `supports_chunked=true` → 收到 `ImageAsync`
5. 大 PNG + `supports_chunked=false` → 收到 `Snapshot`（走兜底提示路径）
6. FileList + `sync_files=true` → 收到 `FilesAsync`
7. FileList + `sync_files=false` → 收到 `Snapshot`

## 建议与回答

- **可以照抄本轮所有代码块**开始落地。
- **唯一的运行时新依赖** = `async-trait = "0.1"`；`Cargo.toml` 一行。
- **零改动区间锁定**：`network/webrtc.rs`、`network/sessions.rs`、前端全部、其他 crate。
- **测试收益**：不启动 Tauri、不建 WebSocket、不真发协议帧就能覆盖 4 分支 + 2 短路共 7 个场景，回归的锚点从"双机手工冒烟"升级为"`cargo test` 秒级完成"。

## 本轮要点

- **两个 trait**：`ClipboardOut`（调用面）、`ClipboardOutProvider`（发现面）
- **快照形态统一 `Arc<ClipboardSnapshot>`**，所有分支共享一份，`.await` 边界 Send 稳定
- **AppHandle 内收到生产实现**，`SyncEngine` 本体不再持有
- **生产端**：`TauriConnectionOut` 包 `ConnectionHandle`、`RegistryOutProvider` 包 `Arc<Mutex<ConnectionRegistry>>`
- **测试端**：`InMemoryOut` 收 `RecordedCall`、`StaticOutsProvider` 直接塞 `Vec`
- **新增依赖**：`async-trait = "0.1"`
- **`lib.rs:3219` 唯一构造点**：新增 3 行、`SyncEngine::new` 参数调整为 `provider`
- **HIGH 点分支等价性表**：现状 4 分支 → 新代码 4 分支一一对应