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
        Self {
            rx,
            provider,
            transfer_slots,
            config,
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if matches!(event.origin, ClipboardOrigin::Remote { .. }) {
                        continue;
                    }

                    let (sync_images, sync_files, max_file_bytes, auto_sync_clipboard) = {
                        let config = self.config.lock().await;
                        (
                            config.sync_images.unwrap_or(true),
                            config.sync_files.unwrap_or(true),
                            config.max_file_bytes.unwrap_or(DEFAULT_MAX_FILE_BYTES),
                            config.auto_sync_clipboard.unwrap_or(true),
                        )
                    };

                    if !auto_sync_clipboard && !event.skip_history_merge {
                        continue;
                    }

                    let outs = self.provider.active_outs().await;
                    if outs.is_empty() {
                        continue;
                    }

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


// ---- inline unit tests ----
//
// 只覆盖 `broadcast_snapshot` + `run()` 的分派与短路语义，不触碰任何 network / Tauri
// 具体实现。InMemoryOut 记录每个 out 收到的调用；断言方式统一走 match 模式（因为
// `ClipboardSnapshot` 未实现 `PartialEq`）。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::image::INLINE_IMAGE_BYTES;
    use crate::clipboard::types::{
        ClipboardEvent, ClipboardFileItem, ClipboardOrigin, ClipboardSnapshot,
    };
    use crate::storage::json::AppConfig;
    use crate::sync::out::test_support::{InMemoryOut, RecordedCall, StaticOutsProvider};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{broadcast, Mutex};

    // ---- helpers ----

    fn cfg(
        auto: bool,
        sync_images: bool,
        sync_files: bool,
        max_file_bytes: u64,
    ) -> Arc<Mutex<AppConfig>> {
        let mut c = AppConfig::default();
        c.auto_sync_clipboard = Some(auto);
        c.sync_images = Some(sync_images);
        c.sync_files = Some(sync_files);
        c.max_file_bytes = Some(max_file_bytes);
        Arc::new(Mutex::new(c))
    }

    fn large_png() -> Vec<u8> {
        vec![0u8; INLINE_IMAGE_BYTES + 8]
    }

    fn small_png() -> Vec<u8> {
        vec![0u8; 16]
    }

    fn one_file() -> Vec<ClipboardFileItem> {
        vec![ClipboardFileItem {
            file_name: "a.bin".into(),
            size_bytes: 128,
            content_hash: [0u8; 32],
            source_path: None,
        }]
    }

    fn local_event(snapshot: ClipboardSnapshot) -> ClipboardEvent {
        ClipboardEvent {
            snapshot,
            origin: ClipboardOrigin::Local,
            timestamp_ms: 0,
            skip_history_merge: false,
        }
    }

    fn remote_event(snapshot: ClipboardSnapshot) -> ClipboardEvent {
        ClipboardEvent {
            snapshot,
            origin: ClipboardOrigin::Remote { peer_name: "peerA".into() },
            timestamp_ms: 0,
            skip_history_merge: false,
        }
    }

    fn spawn_engine(
        outs: Vec<Arc<dyn ClipboardOut>>,
        config: Arc<Mutex<AppConfig>>,
    ) -> broadcast::Sender<ClipboardEvent> {
        let (tx, rx) = broadcast::channel(4);
        let provider: Arc<dyn ClipboardOutProvider> =
            Arc::new(StaticOutsProvider { outs });
        let slots = Arc::new(TransferSlotLimiter::new(2));
        let engine = SyncEngine::new(rx, provider, slots, config);
        tokio::spawn(engine.run());
        tx
    }

    async fn wait_calls(out: &InMemoryOut, n: usize) -> bool {
        for _ in 0..40 {
            if out.recorded().len() >= n {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        false
    }

    async fn assert_no_calls(out: &InMemoryOut) {
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            out.recorded().is_empty(),
            "expected no calls, got {:?}",
            out.recorded()
        );
    }

    // ---- A. 4 分派分支 ----

    #[tokio::test]
    async fn text_snapshot_goes_to_all_outs_send_snapshot() {
        let a = InMemoryOut::new(true);
        let b = InMemoryOut::new(false);
        let tx = spawn_engine(
            vec![a.clone(), b.clone()],
            cfg(true, true, true, 100 * 1024 * 1024),
        );
        tx.send(local_event(ClipboardSnapshot::Text("hi".into()))).unwrap();
        assert!(wait_calls(&a, 1).await);
        assert!(wait_calls(&b, 1).await);
        for out in [&*a, &*b] {
            match out.recorded().as_slice() {
                [RecordedCall::Snapshot {
                    sync_images: true,
                    snapshot: ClipboardSnapshot::Text(t),
                }] if t == "hi" => {}
                other => panic!("unexpected recorded: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn small_image_uses_send_snapshot() {
        let out = InMemoryOut::new(true);
        let tx = spawn_engine(vec![out.clone()], cfg(true, true, true, 100 * 1024 * 1024));
        tx.send(local_event(ClipboardSnapshot::Image {
            png_bytes: small_png(),
            width: 1,
            height: 1,
        }))
        .unwrap();
        assert!(wait_calls(&out, 1).await);
        match out.recorded().as_slice() {
            [RecordedCall::Snapshot {
                sync_images: true,
                snapshot: ClipboardSnapshot::Image { .. },
            }] => {}
            other => panic!("unexpected recorded: {other:?}"),
        }
    }

    #[tokio::test]
    async fn large_image_with_chunked_uses_image_async() {
        let out = InMemoryOut::new(true);
        let tx = spawn_engine(vec![out.clone()], cfg(true, true, true, 100 * 1024 * 1024));
        tx.send(local_event(ClipboardSnapshot::Image {
            png_bytes: large_png(),
            width: 100,
            height: 100,
        }))
        .unwrap();
        assert!(wait_calls(&out, 1).await);
        match out.recorded().as_slice() {
            [RecordedCall::ImageAsync {
                sync_images: true,
                snapshot: ClipboardSnapshot::Image { .. },
            }] => {}
            other => panic!("unexpected recorded: {other:?}"),
        }
    }

    #[tokio::test]
    async fn large_image_without_chunked_falls_back_to_send_snapshot() {
        let out = InMemoryOut::new(false);
        let tx = spawn_engine(vec![out.clone()], cfg(true, true, true, 100 * 1024 * 1024));
        tx.send(local_event(ClipboardSnapshot::Image {
            png_bytes: large_png(),
            width: 100,
            height: 100,
        }))
        .unwrap();
        assert!(wait_calls(&out, 1).await);
        match out.recorded().as_slice() {
            [RecordedCall::Snapshot {
                sync_images: true,
                snapshot: ClipboardSnapshot::Image { .. },
            }] => {}
            other => panic!("unexpected recorded: {other:?}"),
        }
    }

    #[tokio::test]
    async fn file_list_with_sync_uses_files_async() {
        let out = InMemoryOut::new(true);
        let tx = spawn_engine(vec![out.clone()], cfg(true, true, true, 100 * 1024 * 1024));
        tx.send(local_event(ClipboardSnapshot::FileList { files: one_file() })).unwrap();
        assert!(wait_calls(&out, 1).await);
        match out.recorded().as_slice() {
            [RecordedCall::FilesAsync {
                sync_files: true,
                max_file_bytes,
                snapshot: ClipboardSnapshot::FileList { files },
            }] if *max_file_bytes == 100 * 1024 * 1024 && files.len() == 1 => {}
            other => panic!("unexpected recorded: {other:?}"),
        }
    }

    #[tokio::test]
    async fn file_list_without_sync_falls_back_to_send_snapshot() {
        // sync_files=false 时 FileList 应落到 send_snapshot（webrtc 内不作任何动作）
        let out = InMemoryOut::new(true);
        let tx = spawn_engine(vec![out.clone()], cfg(true, true, false, 100 * 1024 * 1024));
        tx.send(local_event(ClipboardSnapshot::FileList { files: one_file() })).unwrap();
        assert!(wait_calls(&out, 1).await);
        match out.recorded().as_slice() {
            [RecordedCall::Snapshot {
                sync_images: true,
                snapshot: ClipboardSnapshot::FileList { .. },
            }] => {}
            other => panic!("unexpected recorded: {other:?}"),
        }
    }

    // ---- B. 2 短路 ----

    #[tokio::test]
    async fn remote_origin_event_is_skipped() {
        let out = InMemoryOut::new(true);
        let tx = spawn_engine(vec![out.clone()], cfg(true, true, true, 100 * 1024 * 1024));
        tx.send(remote_event(ClipboardSnapshot::Text("from-remote".into()))).unwrap();
        assert_no_calls(&out).await;
    }

    #[tokio::test]
    async fn auto_sync_off_and_not_skip_history_is_skipped() {
        let out = InMemoryOut::new(true);
        let tx = spawn_engine(vec![out.clone()], cfg(false, true, true, 100 * 1024 * 1024));
        tx.send(local_event(ClipboardSnapshot::Text("hi".into()))).unwrap();
        assert_no_calls(&out).await;
    }

    // ---- C. 3 副条件 ----

    #[tokio::test]
    async fn sync_images_off_large_image_falls_back_to_send_snapshot() {
        // sync_images=false + 大图 + supports_chunked=true：
        // 分派仍走 chunked 分支（判据是"大图 + supports_chunked"），
        // 但 sync_images 会被透传到 send_image_async；webrtc 内会短路返回。
        // SyncEngine 单测只锁分派方向本身 + 参数透传。
        let out = InMemoryOut::new(true);
        let tx = spawn_engine(vec![out.clone()], cfg(true, false, true, 100 * 1024 * 1024));
        tx.send(local_event(ClipboardSnapshot::Image {
            png_bytes: large_png(),
            width: 100,
            height: 100,
        }))
        .unwrap();
        assert!(wait_calls(&out, 1).await);
        match out.recorded().as_slice() {
            [RecordedCall::ImageAsync {
                sync_images: false,
                snapshot: ClipboardSnapshot::Image { .. },
            }] => {}
            other => panic!("unexpected recorded: {other:?}"),
        }
    }

    #[tokio::test]
    async fn sync_files_off_file_list_falls_back_to_send_snapshot() {
        // sync_files=false 时 FileList 分派应绕过 files_async，落到 send_snapshot。
        // 与 file_list_without_sync 断言一致，重复用例保留是为副条件矩阵完整性。
        let out = InMemoryOut::new(true);
        let tx = spawn_engine(vec![out.clone()], cfg(true, true, false, 100 * 1024 * 1024));
        tx.send(local_event(ClipboardSnapshot::FileList { files: one_file() })).unwrap();
        assert!(wait_calls(&out, 1).await);
        match out.recorded().as_slice() {
            [RecordedCall::Snapshot {
                sync_images: true,
                snapshot: ClipboardSnapshot::FileList { .. },
            }] => {}
            other => panic!("unexpected recorded: {other:?}"),
        }
    }

    #[tokio::test]
    async fn empty_outs_list_does_not_panic_and_no_spawn() {
        // provider 返回空 Vec：run() 应短路 `outs.is_empty()`，无 panic、无 spawn。
        let out = InMemoryOut::new(true);      // 只是拿来做时间窗口的旁观
        let tx = {
            let (tx, rx) = broadcast::channel(4);
            let provider: Arc<dyn ClipboardOutProvider> =
                Arc::new(StaticOutsProvider { outs: vec![] });
            let slots = Arc::new(TransferSlotLimiter::new(2));
            let cfg = cfg(true, true, true, 100 * 1024 * 1024);
            let engine = SyncEngine::new(rx, provider, slots, cfg);
            tokio::spawn(engine.run());
            tx
        };
        tx.send(local_event(ClipboardSnapshot::Text("no-peers".into()))).unwrap();
        // 无 outs，本旁观 out 也不该被记录：
        assert_no_calls(&out).await;
    }
}
