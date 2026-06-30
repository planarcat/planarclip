use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

/// Single entry for sync failure UX: in-app notice (`StatusNotice`) + system notification.
pub fn notify_sync_failure(app_handle: Option<&AppHandle>, message: &str) {
    let message = message.trim();
    if message.is_empty() {
        return;
    }

    if let Some(app_handle) = app_handle {
        let _ = app_handle.emit(
            "clipboard-sync-activity",
            serde_json::json!({
                "active": false,
                "kind": "notice",
                "message": message,
            }),
        );
        crate::window::send_user_notification(app_handle, message, true);
        tracing::info!("sync failure notified: {message}");
    }
}

pub fn emit_sync_activity(app_handle: Option<&AppHandle>, active: bool, kind: &str, message: &str) {
    let kind_static: &'static str = match kind {
        "image" => "image",
        "notice" => "notice",
        _ => "file",
    };
    emit_sync_progress(
        app_handle,
        SyncProgressUpdate {
            active,
            kind: kind_static,
            message: message.to_string(),
            progress: None,
            direction: None,
            label: None,
            bytes_done: None,
            bytes_total: None,
            batch_index: None,
            batch_total: None,
        },
        &mut ProgressThrottle::default(),
        true,
    );
}

#[derive(Clone, Debug)]
pub struct SyncProgressUpdate {
    pub active: bool,
    pub kind: &'static str,
    pub message: String,
    pub progress: Option<f64>,
    pub direction: Option<&'static str>,
    pub label: Option<String>,
    pub bytes_done: Option<u64>,
    pub bytes_total: Option<u64>,
    pub batch_index: Option<u32>,
    pub batch_total: Option<u32>,
}

#[derive(Default)]
struct ProgressThrottle {
    last_emit_ms: u64,
    last_progress_pct: u32,
}

impl ProgressThrottle {
    fn should_emit(&mut self, progress: f64, force: bool) -> bool {
        let pct = (progress.clamp(0.0, 1.0) * 100.0).round() as u32;
        if force || pct == 0 || pct >= 100 {
            self.last_progress_pct = pct;
            self.last_emit_ms = now_ms();
            return true;
        }
        if pct.abs_diff(self.last_progress_pct) >= 1 {
            self.last_progress_pct = pct;
            self.last_emit_ms = now_ms();
            return true;
        }
        let now = now_ms();
        if now.saturating_sub(self.last_emit_ms) >= 150 {
            self.last_progress_pct = pct;
            self.last_emit_ms = now;
            return true;
        }
        false
    }
}

pub fn emit_sync_progress(
    app_handle: Option<&AppHandle>,
    update: SyncProgressUpdate,
    throttle: &mut ProgressThrottle,
    force: bool,
) {
    let Some(app_handle) = app_handle else {
        return;
    };

    if update.active {
        if let Some(progress) = update.progress {
            if !throttle.should_emit(progress, force) {
                return;
            }
        }
    }

    let _ = app_handle.emit(
        "clipboard-sync-activity",
        serde_json::json!({
            "active": update.active,
            "kind": update.kind,
            "message": update.message,
            "progress": update.progress,
            "direction": update.direction,
            "label": update.label,
            "bytes_done": update.bytes_done,
            "bytes_total": update.bytes_total,
            "batch_index": update.batch_index,
            "batch_total": update.batch_total,
        }),
    );
}

struct BatchSendContext {
    batch_total: u32,
    batch_bytes_total: u64,
    bytes_completed: u64,
}

pub struct TransferProgressReporter {
    app_handle: Option<AppHandle>,
    kind: &'static str,
    direction: &'static str,
    label: String,
    batch_index: Option<u32>,
    batch: Option<BatchSendContext>,
    throttle: ProgressThrottle,
}

impl TransferProgressReporter {
    pub fn file_send(
        app_handle: Option<AppHandle>,
        label: String,
        batch_index: Option<u32>,
        batch_total: Option<u32>,
        batch_bytes_total: u64,
        bytes_completed: u64,
    ) -> Self {
        let batch = batch_total.filter(|total| *total > 1).map(|batch_total| BatchSendContext {
            batch_total,
            batch_bytes_total,
            bytes_completed,
        });
        Self {
            app_handle,
            kind: "file",
            direction: "send",
            label,
            batch_index,
            batch,
            throttle: ProgressThrottle::default(),
        }
    }

    pub fn file_receive(
        app_handle: Option<AppHandle>,
        label: String,
        batch_index: Option<u32>,
        batch_total: Option<u32>,
        batch_bytes_total: u64,
        bytes_completed: u64,
    ) -> Self {
        let batch = batch_total.filter(|total| *total > 1).map(|batch_total| BatchSendContext {
            batch_total,
            batch_bytes_total,
            bytes_completed,
        });
        Self {
            app_handle,
            kind: "file",
            direction: "receive",
            label,
            batch_index,
            batch,
            throttle: ProgressThrottle::default(),
        }
    }

    pub fn image_send(app_handle: Option<AppHandle>, total_bytes: u64) -> Self {
        Self {
            app_handle,
            kind: "image",
            direction: "send",
            label: "图片".to_string(),
            batch_index: None,
            batch: Some(BatchSendContext {
                batch_total: 1,
                batch_bytes_total: total_bytes,
                bytes_completed: 0,
            }),
            throttle: ProgressThrottle::default(),
        }
    }

    pub fn image_receive(app_handle: Option<AppHandle>, total_bytes: u64) -> Self {
        Self {
            app_handle,
            kind: "image",
            direction: "receive",
            label: "图片".to_string(),
            batch_index: None,
            batch: Some(BatchSendContext {
                batch_total: 1,
                batch_bytes_total: total_bytes,
                bytes_completed: 0,
            }),
            throttle: ProgressThrottle::default(),
        }
    }

    pub fn set_batch_index(&mut self, batch_index: Option<u32>) {
        self.batch_index = batch_index;
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
    }

    pub fn report_bytes(&mut self, bytes_done: u64, bytes_total: u64, force: bool) {
        let bytes_total = bytes_total.max(1);
        let progress = bytes_done as f64 / bytes_total as f64;
        let message = format_transfer_message(
            self.direction,
            &self.label,
            self.batch.as_ref().map(|batch| batch.batch_total),
            Some((progress * 100.0).round() as u32),
        );
        emit_sync_progress(
            self.app_handle.as_ref(),
            SyncProgressUpdate {
                active: true,
                kind: self.kind,
                message,
                progress: Some(progress),
                direction: Some(self.direction),
                label: Some(self.label.clone()),
                bytes_done: Some(bytes_done),
                bytes_total: Some(bytes_total),
                batch_index: self.batch_index,
                batch_total: self.batch.as_ref().map(|batch| batch.batch_total),
            },
            &mut self.throttle,
            force,
        );
    }

    pub fn report_chunks(&mut self, acked: u32, chunk_total: u32, file_total_bytes: u64, force: bool) {
        if chunk_total == 0 {
            return;
        }
        let file_done = ((acked as f64 / chunk_total as f64) * file_total_bytes as f64).min(file_total_bytes as f64)
            as u64;
        let (bytes_done, bytes_total) = if let Some(batch) = &self.batch {
            (
                batch.bytes_completed.saturating_add(file_done),
                batch.batch_bytes_total.max(1),
            )
        } else {
            (file_done, file_total_bytes.max(1))
        };
        let progress = bytes_done as f64 / bytes_total as f64;
        let message = format_transfer_message(
            self.direction,
            &self.label,
            self.batch.as_ref().map(|batch| batch.batch_total),
            Some((progress * 100.0).round() as u32),
        );
        emit_sync_progress(
            self.app_handle.as_ref(),
            SyncProgressUpdate {
                active: true,
                kind: self.kind,
                message,
                progress: Some(progress),
                direction: Some(self.direction),
                label: Some(self.label.clone()),
                bytes_done: Some(bytes_done),
                bytes_total: Some(bytes_total),
                batch_index: self.batch_index,
                batch_total: self.batch.as_ref().map(|batch| batch.batch_total),
            },
            &mut self.throttle,
            force,
        );
    }

    pub fn complete_file_in_batch(&mut self, file_bytes: u64) {
        if let Some(batch) = &mut self.batch {
            batch.bytes_completed = batch.bytes_completed.saturating_add(file_bytes);
        }
    }

    pub fn finish(&mut self, message: &str) {
        emit_sync_progress(
            self.app_handle.as_ref(),
            SyncProgressUpdate {
                active: false,
                kind: self.kind,
                message: message.to_string(),
                progress: Some(1.0),
                direction: Some(self.direction),
                label: Some(self.label.clone()),
                bytes_done: self.batch.as_ref().map(|batch| batch.batch_bytes_total),
                bytes_total: self.batch.as_ref().map(|batch| batch.batch_bytes_total),
                batch_index: self.batch_index,
                batch_total: self.batch.as_ref().map(|batch| batch.batch_total),
            },
            &mut self.throttle,
            true,
        );
    }
}

pub fn format_transfer_message(
    direction: &str,
    label: &str,
    batch_total: Option<u32>,
    progress_pct: Option<u32>,
) -> String {
    let verb = if direction == "send" {
        "发送"
    } else {
        "接收"
    };
    let pct_suffix = progress_pct
        .map(|pct| format!(" {pct}%"))
        .unwrap_or_default();
    if let Some(batch_total) = batch_total.filter(|total| *total > 1) {
        format!("正在{verb} {label} 等 {batch_total} 个文件…{pct_suffix}")
    } else {
        format!("正在{verb} {label}…{pct_suffix}")
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
