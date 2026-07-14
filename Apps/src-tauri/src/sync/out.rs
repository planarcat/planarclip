//! ClipboardOut / ClipboardOutProvider — SyncEngine 的可测抽象边界。
//!
//! 生产实现 `TauriConnectionOut` + `RegistryOutProvider` 封装 `ConnectionHandle`
//! 与 `ConnectionRegistry`，`AppHandle` 内收到实现内部，SyncEngine 本体不再感知
//! Tauri。测试端（Phase 3）用 `test_support::InMemoryOut` +
//! `test_support::StaticOutsProvider` 直接构造 outs 列表进行分派逻辑单测。

use std::sync::Arc;

use async_trait::async_trait;
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::clipboard::types::ClipboardSnapshot;
use crate::network::sessions::ConnectionRegistry;
use crate::network::webrtc::ConnectionHandle;

/// SyncEngine 面向对端的对外输出通道抽象。
///
/// 4 个方法与 `ConnectionHandle` 现有 API 语义一一对齐，唯一差异：
/// - 快照统一为 `Arc<ClipboardSnapshot>`，四个分支共享一份克隆；
/// - `AppHandle` 不再作为入参，生产实现内部自持。
#[async_trait]
pub trait ClipboardOut: Send + Sync {
    /// 是否支持分块图片（大图必须走此能力，否则走兜底提示）。
    fn supports_chunked_images(&self) -> bool;

    /// 同步分派：文本 / 内联小图 / 空 / 大图但不支持分块的兜底提示 / 关闭同步的文件回落。
    fn send_snapshot(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool);

    /// 异步分块发送图片（仅当 `supports_chunked_images` 为 true 时被调用）。
    async fn send_image_async(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool);

    /// 异步文件批传送。
    async fn send_files_async(
        &self,
        snapshot: Arc<ClipboardSnapshot>,
        sync_files: bool,
        max_file_bytes: u64,
    );
}

/// 发现面：SyncEngine 每次事件调用一次，返回当前所有活跃的输出通道。
#[async_trait]
pub trait ClipboardOutProvider: Send + Sync {
    async fn active_outs(&self) -> Vec<Arc<dyn ClipboardOut>>;
}

/// 生产实现：把 `ConnectionHandle` 包装成 `ClipboardOut`。
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

/// 生产实现：从 `ConnectionRegistry` 拉取活跃 handles，包装为 outs。
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

// ---- test support ----
//
// 仅在 `cargo test` 场景暴露：InMemoryOut 记录调用序列，StaticOutsProvider
// 让 SyncEngine 单测能塞入固定 out 列表。生产不感知这个模块。
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// SyncEngine 会调 out 的哪个方法，用一个枚举拍平记录，便于测试断言。
    #[derive(Debug, Clone)]
    pub enum RecordedCall {
        Snapshot {
            sync_images: bool,
            snapshot: ClipboardSnapshot,
        },
        ImageAsync {
            sync_images: bool,
            snapshot: ClipboardSnapshot,
        },
        FilesAsync {
            sync_files: bool,
            max_file_bytes: u64,
            snapshot: ClipboardSnapshot,
        },
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
        fn supports_chunked_images(&self) -> bool {
            self.supports_chunked
        }

        fn send_snapshot(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool) {
            self.calls.lock().unwrap().push(RecordedCall::Snapshot {
                sync_images,
                snapshot: (*snapshot).clone(),
            });
        }

        async fn send_image_async(&self, snapshot: Arc<ClipboardSnapshot>, sync_images: bool) {
            self.calls.lock().unwrap().push(RecordedCall::ImageAsync {
                sync_images,
                snapshot: (*snapshot).clone(),
            });
        }

        async fn send_files_async(
            &self,
            snapshot: Arc<ClipboardSnapshot>,
            sync_files: bool,
            max_file_bytes: u64,
        ) {
            self.calls.lock().unwrap().push(RecordedCall::FilesAsync {
                sync_files,
                max_file_bytes,
                snapshot: (*snapshot).clone(),
            });
        }
    }

    /// 直接把一批预制的 outs 塞进 SyncEngine。
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
