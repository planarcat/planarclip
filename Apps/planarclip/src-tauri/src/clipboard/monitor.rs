use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use arboard::ImageData;
use tokio::sync::broadcast;

use crate::clipboard::file::{file_list_hash, snapshot_from_file_paths, DEFAULT_MAX_FILE_BYTES, MAX_BATCH_BYTES};
use crate::clipboard::image::{encode_rgba_to_png, snapshot_from_png_bytes};
#[cfg(not(windows))]
use crate::clipboard::image::decode_png_to_rgba;
use crate::clipboard::types::{ClipboardEvent, ClipboardSnapshot};

static SELF_WRITING: AtomicBool = AtomicBool::new(false);
static SUPPRESSED_REMOTE_WRITE: Mutex<Option<SuppressedRemoteWrite>> = Mutex::new(None);
const CLIPBOARD_POLL_INTERVAL_MS: u64 = 150;
const REMOTE_WRITE_SUPPRESSION_MS: u64 = 1_500;

struct SuppressedRemoteWrite {
    hash: [u8; 32],
    until_ms: u64,
}

pub struct ClipboardMonitor {
    tx: broadcast::Sender<ClipboardEvent>,
    last_hash: [u8; 32],
    last_read_error: Option<String>,
    #[cfg(windows)]
    last_clipboard_seq: Option<u32>,
}

impl ClipboardMonitor {
    pub fn new(tx: broadcast::Sender<ClipboardEvent>) -> Self {
        Self {
            tx,
            last_hash: [0u8; 32],
            last_read_error: None,
            #[cfg(windows)]
            last_clipboard_seq: None,
        }
    }

    pub fn set_self_writing(flag: bool) {
        SELF_WRITING.store(flag, Ordering::SeqCst);
    }

    pub fn is_self_writing() -> bool {
        SELF_WRITING.load(Ordering::SeqCst)
    }

    pub async fn run(&mut self) {
        self.run_polling_loop().await;
    }

    async fn run_polling_loop(&mut self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(CLIPBOARD_POLL_INTERVAL_MS));
        loop {
            interval.tick().await;
            self.capture_clipboard_change();
        }
    }

    fn capture_clipboard_change(&mut self) {
        if Self::is_self_writing() {
            return;
        }

        #[cfg(windows)]
        if !Self::clipboard_sequence_changed(self.last_clipboard_seq) {
            return;
        }

        match Self::read_clipboard() {
            Ok(snapshot) => {
                self.last_read_error.take();

                if snapshot.is_empty() {
                    #[cfg(windows)]
                    if Self::image_read_pending() {
                        tracing::debug!(
                            "clipboard image format present but read not ready, will retry"
                        );
                        return;
                    }
                    self.note_observed_clipboard_sequence();
                    self.last_hash = [0u8; 32];
                    return;
                }

                let hash = snapshot.content_hash();
                if Self::should_suppress_local_emit(hash) {
                    self.track_clipboard_state(hash);
                    return;
                }

                if self.should_emit_clipboard_change(hash) {
                    self.track_clipboard_state(hash);
                    let _ = self.tx.send(ClipboardEvent::local(snapshot));
                } else {
                    self.note_observed_clipboard_sequence();
                }
            }
            Err(error) => {
                #[cfg(windows)]
                if Self::image_read_pending() || Self::file_read_pending() {
                    tracing::debug!(
                        "clipboard read failed while image/file format present, will retry: {error}"
                    );
                    return;
                }
                self.note_observed_clipboard_sequence();
                if self.last_read_error.as_deref() != Some(error.as_str()) {
                    self.last_read_error = Some(error.clone());
                }
            }
        }
    }

    fn read_clipboard() -> Result<ClipboardSnapshot, String> {
        #[cfg(windows)]
        if crate::platform::windows::clipboard::has_file_format() {
            if let Some(paths) = crate::platform::windows::clipboard::read_file_paths() {
                match snapshot_from_file_paths(
                    paths,
                    DEFAULT_MAX_FILE_BYTES,
                    MAX_BATCH_BYTES,
                ) {
                    Ok(snapshot) if !snapshot.is_empty() => return Ok(snapshot),
                    Ok(_) => {}
                    Err(error) => return Err(error),
                }
            }
        }

        #[cfg(windows)]
        if let Some(snapshot) = crate::platform::windows::clipboard::read_snapshot() {
            return Ok(snapshot);
        }

        let mut clipboard =
            arboard::Clipboard::new().map_err(|error| format!("clipboard init failed: {error}"))?;

        match clipboard.get_image() {
            Ok(image) => {
                if let Some(snapshot) = Self::snapshot_from_image(&image) {
                    return Ok(snapshot);
                }
                tracing::info!("arboard image present but could not be normalized for sync");
            }
            Err(error) => {
                tracing::debug!("arboard get_image unavailable: {error}");
            }
        }

        match clipboard.get_text() {
            Ok(text) if !text.is_empty() => Ok(ClipboardSnapshot::Text(text)),
            Ok(_) => Ok(ClipboardSnapshot::Empty),
            Err(error) => Err(format!("clipboard read failed: {error}")),
        }
    }

    fn snapshot_from_image(image: &ImageData<'_>) -> Option<ClipboardSnapshot> {
        let png_bytes = encode_rgba_to_png(image).ok()?;
        snapshot_from_png_bytes(png_bytes)
    }

    fn should_emit_clipboard_change(&self, hash: [u8; 32]) -> bool {
        if hash != self.last_hash {
            return true;
        }

        #[cfg(windows)]
        {
            let Some(seq) = Self::current_clipboard_seq() else {
                return false;
            };
            return self.last_clipboard_seq != Some(seq);
        }

        #[cfg(not(windows))]
        false
    }

    fn track_clipboard_state(&mut self, hash: [u8; 32]) {
        self.last_hash = hash;
        self.note_observed_clipboard_sequence();
    }

    fn note_observed_clipboard_sequence(&mut self) {
        #[cfg(windows)]
        if let Some(seq) = Self::current_clipboard_seq() {
            self.last_clipboard_seq = Some(seq);
        }
    }

    #[cfg(windows)]
    fn clipboard_sequence_changed(last_seq: Option<u32>) -> bool {
        let Some(seq) = Self::current_clipboard_seq() else {
            return true;
        };
        last_seq != Some(seq)
    }

    #[cfg(windows)]
    fn current_clipboard_seq() -> Option<u32> {
        crate::platform::windows::clipboard::current_sequence()
    }

    #[cfg(windows)]
    fn image_read_pending() -> bool {
        crate::platform::windows::clipboard::has_image_format()
    }

    fn should_suppress_local_emit(hash: [u8; 32]) -> bool {
        let now_ms = now_ms();
        let Ok(mut state) = SUPPRESSED_REMOTE_WRITE.lock() else {
            return false;
        };

        match state.as_ref() {
            Some(suppressed) if suppressed.until_ms >= now_ms && suppressed.hash == hash => true,
            Some(suppressed) if suppressed.until_ms < now_ms => {
                *state = None;
                false
            }
            _ => false,
        }
    }

    #[cfg(windows)]
    fn file_read_pending() -> bool {
        crate::platform::windows::clipboard::has_file_format()
    }

    pub fn write_clipboard_files(paths: &[std::path::PathBuf]) {
        if paths.is_empty() {
            return;
        }

        let files: Vec<_> = paths
            .iter()
            .filter_map(|path| {
                let metadata = std::fs::metadata(path).ok()?;
                let content_hash = crate::clipboard::file::hash_file(path).ok()?;
                Some(crate::clipboard::types::ClipboardFileItem {
                    file_name: path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("file")
                        .to_string(),
                    size_bytes: metadata.len(),
                    content_hash,
                    source_path: Some(path.clone()),
                })
            })
            .collect();

        if files.is_empty() {
            return;
        }

        let hash = file_list_hash(&files);
        Self::register_suppressed_write(hash);
        Self::set_self_writing(true);

        #[cfg(windows)]
        {
            let paths = paths.to_vec();
            if let Err(error) = std::thread::Builder::new()
                .name("planarclip-clip-write".into())
                .spawn(move || crate::platform::windows::clipboard::write_file_paths(&paths))
                .map_err(|error| format!("spawn clipboard thread failed: {error}"))
                .and_then(|handle| {
                    handle
                        .join()
                        .map_err(|_| "clipboard write thread panicked".to_string())?
                })
            {
                tracing::warn!("failed to write clipboard files: {error}");
            }
        }

        #[cfg(not(windows))]
        {
            let _ = paths;
            tracing::warn!("clipboard file write is not supported on this platform yet");
        }

        Self::set_self_writing(false);
    }

    pub fn write_clipboard(text: &str) {
        let hash = *blake3::hash(text.as_bytes()).as_bytes();
        Self::register_suppressed_write(hash);
        Self::set_self_writing(true);
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
        Self::set_self_writing(false);
    }

    pub fn write_clipboard_image(png_bytes: &[u8], width: u32, height: u32) {
        let hash = *blake3::hash(png_bytes).as_bytes();
        Self::register_suppressed_write(hash);
        Self::set_self_writing(true);

        if let Err(error) = Self::write_clipboard_image_inner(png_bytes, width, height) {
            tracing::warn!("failed to write clipboard image: {error}");
        }

        Self::set_self_writing(false);
    }

    fn write_clipboard_image_inner(png_bytes: &[u8], width: u32, height: u32) -> Result<(), String> {
        #[cfg(windows)]
        {
            let png_bytes = png_bytes.to_vec();
            let handle = std::thread::Builder::new()
                .name("planarclip-clip-write".into())
                .spawn(move || crate::platform::windows::clipboard::write_image(&png_bytes, width, height))
                .map_err(|error| format!("spawn clipboard thread failed: {error}"))?;
            handle
                .join()
                .map_err(|_| "clipboard write thread panicked".to_string())?
        }

        #[cfg(not(windows))]
        {
            let (decoded_width, decoded_height, rgba) = decode_png_to_rgba(png_bytes)?;
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|error| format!("clipboard init failed: {error}"))?;
            let image = ImageData {
                width: decoded_width as usize,
                height: decoded_height as usize,
                bytes: rgba.into(),
            };
            clipboard
                .set_image(image)
                .map_err(|error| format!("set_image failed: {error}"))
        }
    }

    fn register_suppressed_write(hash: [u8; 32]) {
        if let Ok(mut state) = SUPPRESSED_REMOTE_WRITE.lock() {
            *state = Some(SuppressedRemoteWrite {
                hash,
                until_ms: now_ms() + REMOTE_WRITE_SUPPRESSION_MS,
            });
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
