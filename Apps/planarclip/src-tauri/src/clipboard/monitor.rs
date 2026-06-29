use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

use arboard::ImageData;
use tokio::sync::{broadcast, mpsc, Mutex as AsyncMutex};

use crate::clipboard::file::{
    file_list_as_sync_text, file_list_hash, snapshot_from_file_paths,
    snapshot_from_file_paths_meta, DEFAULT_MAX_FILE_BYTES, MAX_BATCH_BYTES,
};
use crate::clipboard::image::{encode_rgba_to_png, snapshot_from_png_bytes};
#[cfg(not(windows))]
use crate::clipboard::image::decode_png_to_rgba;
use crate::clipboard::types::{ClipboardEvent, ClipboardSnapshot};
use crate::storage::json::AppConfig;

static SELF_WRITING: AtomicBool = AtomicBool::new(false);
static SUPPRESSED_REMOTE_WRITE: Mutex<Option<SuppressedRemoteWrite>> = Mutex::new(None);
const CLIPBOARD_POLL_INTERVAL_MS: u64 = 150;
const REMOTE_WRITE_SUPPRESSION_MS: u64 = 1_500;

struct SuppressedRemoteWrite {
    hash: [u8; 32],
    until_ms: u64,
}

/// Dedup baseline captured when history is cleared so the monitor can resume without re-emitting.
#[derive(Clone, Copy, Debug)]
pub struct ClipboardDedupBaseline {
    pub last_hash: [u8; 32],
    #[cfg(windows)]
    pub last_clipboard_seq: Option<u32>,
}

pub struct ClipboardMonitor {
    tx: broadcast::Sender<ClipboardEvent>,
    config: Arc<AsyncMutex<AppConfig>>,
    reset_generation: Arc<AtomicU64>,
    dedup_baseline: Arc<AsyncMutex<Option<ClipboardDedupBaseline>>>,
    last_reset_generation: u64,
    last_hash: [u8; 32],
    last_read_error: Option<String>,
    #[cfg(windows)]
    last_clipboard_seq: Option<u32>,
}

impl ClipboardMonitor {
    pub fn new(
        tx: broadcast::Sender<ClipboardEvent>,
        config: Arc<AsyncMutex<AppConfig>>,
        reset_generation: Arc<AtomicU64>,
        dedup_baseline: Arc<AsyncMutex<Option<ClipboardDedupBaseline>>>,
    ) -> Self {
        let initial_generation = reset_generation.load(Ordering::SeqCst);
        Self {
            tx,
            config,
            reset_generation,
            dedup_baseline,
            last_reset_generation: initial_generation,
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

    pub fn capture_dedup_baseline(max_file_bytes: u64, sync_files: bool) -> ClipboardDedupBaseline {
        let last_hash = match Self::read_clipboard(max_file_bytes, sync_files) {
            Ok(snapshot) if !snapshot.is_empty() => snapshot.content_hash(),
            _ => [0u8; 32],
        };

        ClipboardDedupBaseline {
            last_hash,
            #[cfg(windows)]
            last_clipboard_seq: Self::current_clipboard_seq(),
        }
    }

    pub async fn run(&mut self) {
        #[cfg(windows)]
        {
            match self.run_windows_listener().await {
                Ok(()) => return,
                Err(error) => {
                    tracing::warn!(
                        "clipboard event listener unavailable, falling back to polling: {error}"
                    );
                }
            }
        }

        self.run_polling_loop().await;
    }

    #[cfg(windows)]
    async fn run_windows_listener(&mut self) -> Result<(), String> {
        let (signal_tx, mut signal_rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);

        std::thread::spawn(move || windows_listener::run(signal_tx, ready_tx));

        match ready_rx.recv() {
            Ok(Ok(())) => {
                tracing::info!("clipboard event listener started");
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => {
                return Err("clipboard listener startup channel closed unexpectedly".to_string());
            }
        }

        let mut pending_retry =
            tokio::time::interval(std::time::Duration::from_millis(CLIPBOARD_POLL_INTERVAL_MS));
        pending_retry.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut retry_pending = false;

        loop {
            tokio::select! {
                signal = signal_rx.recv() => {
                    match signal {
                        Some(ClipboardListenerSignal::ClipboardChanged) => {
                            retry_pending = self.capture_clipboard_change().await;
                        }
                        Some(ClipboardListenerSignal::Failed(error)) => return Err(error),
                        None => return Err("clipboard listener exited unexpectedly".to_string()),
                    }
                }
                _ = pending_retry.tick(), if retry_pending => {
                    retry_pending = self.capture_clipboard_change().await;
                }
            }
        }
    }

    async fn run_polling_loop(&mut self) {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(CLIPBOARD_POLL_INTERVAL_MS));
        loop {
            interval.tick().await;
            let _ = self.capture_clipboard_change().await;
        }
    }

    /// Returns `true` when clipboard content may not be ready yet and should be retried.
    async fn capture_clipboard_change(&mut self) -> bool {
        if Self::is_self_writing() {
            return false;
        }

        self.apply_reset_if_needed().await;

        #[cfg(windows)]
        if !Self::clipboard_sequence_changed(self.last_clipboard_seq) {
            return false;
        }

        let (sync_files, max_file_bytes) = {
            let config = self.config.lock().await;
            (
                config.sync_files.unwrap_or(true),
                config.max_file_bytes.unwrap_or(DEFAULT_MAX_FILE_BYTES),
            )
        };

        match Self::read_clipboard(max_file_bytes, sync_files) {
            Ok(snapshot) => {
                self.last_read_error.take();

                if snapshot.is_empty() {
                    #[cfg(windows)]
                    if Self::image_read_pending() {
                        tracing::debug!(
                            "clipboard image format present but read not ready, will retry"
                        );
                        return true;
                    }
                    self.note_observed_clipboard_sequence();
                    self.last_hash = [0u8; 32];
                    return false;
                }

                let hash = snapshot.content_hash();
                if Self::should_suppress_local_emit(hash) {
                    self.track_clipboard_state(hash);
                    return false;
                }

                if self.should_emit_clipboard_change(hash) {
                    self.track_clipboard_state(hash);
                    let _ = self.tx.send(ClipboardEvent::local(snapshot));
                } else {
                    self.note_observed_clipboard_sequence();
                }

                false
            }
            Err(error) => {
                #[cfg(windows)]
                if Self::image_read_pending() || Self::file_read_pending() {
                    tracing::debug!(
                        "clipboard read failed while image/file format present, will retry: {error}"
                    );
                    return true;
                }
                self.note_observed_clipboard_sequence();
                if self.last_read_error.as_deref() != Some(error.as_str()) {
                    self.last_read_error = Some(error.clone());
                }
                false
            }
        }
    }

    async fn apply_reset_if_needed(&mut self) {
        let current_generation = self.reset_generation.load(Ordering::SeqCst);
        if current_generation == self.last_reset_generation {
            return;
        }

        self.last_reset_generation = current_generation;
        self.last_read_error = None;

        if let Some(baseline) = self.dedup_baseline.lock().await.take() {
            self.last_hash = baseline.last_hash;
            #[cfg(windows)]
            {
                self.last_clipboard_seq = baseline.last_clipboard_seq;
            }
        }
    }

    fn read_clipboard(max_file_bytes: u64, sync_files: bool) -> Result<ClipboardSnapshot, String> {
        #[cfg(windows)]
        if crate::platform::windows::clipboard::has_file_format() {
            if let Some(paths) = crate::platform::windows::clipboard::read_file_paths() {
                if !sync_files {
                    match snapshot_from_file_paths_meta(paths) {
                        Ok(ClipboardSnapshot::FileList { files }) => {
                            if let Some(text) = file_list_as_sync_text(&files) {
                                return Ok(ClipboardSnapshot::Text(text));
                            }
                            return Ok(ClipboardSnapshot::Empty);
                        }
                        Ok(snapshot) => return Ok(snapshot),
                        Err(error) => return Err(error),
                    }
                }

                match snapshot_from_file_paths(paths, max_file_bytes, MAX_BATCH_BYTES) {
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

#[cfg(windows)]
enum ClipboardListenerSignal {
    ClipboardChanged,
    Failed(String),
}

#[cfg(windows)]
mod windows_listener {
    use std::ffi::c_void;

    use tokio::sync::mpsc;
    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::DataExchange::{
        AddClipboardFormatListener, RemoveClipboardFormatListener,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
        RegisterClassW, SetWindowLongPtrW, TranslateMessage, CREATESTRUCTW, GWLP_USERDATA,
        HWND_MESSAGE, MSG, WM_CLIPBOARDUPDATE, WM_NCCREATE, WM_NCDESTROY, WNDCLASSW,
    };

    use super::ClipboardListenerSignal;

    pub fn run(
        signal_tx: mpsc::UnboundedSender<ClipboardListenerSignal>,
        ready_tx: std::sync::mpsc::SyncSender<Result<(), String>>,
    ) {
        if let Err(error) = run_inner(signal_tx, ready_tx) {
            tracing::warn!("clipboard event listener exited: {error}");
        }
    }

    fn run_inner(
        signal_tx: mpsc::UnboundedSender<ClipboardListenerSignal>,
        ready_tx: std::sync::mpsc::SyncSender<Result<(), String>>,
    ) -> Result<(), String> {
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        if instance.is_null() {
            let error = format!("load module handle failed: {}", unsafe { GetLastError() });
            let _ = ready_tx.send(Err(error.clone()));
            return Err(error);
        }

        let class_name = wide_null("PlanarClipClipboardListenerWindow");
        let window_name = wide_null("PlanarClipClipboardListener");
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(listener_wndproc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..unsafe { std::mem::zeroed() }
        };

        unsafe {
            RegisterClassW(&window_class);
        }

        let listener_ptr = Box::into_raw(Box::new(signal_tx));
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                instance,
                listener_ptr.cast::<c_void>(),
            )
        };

        if hwnd.is_null() {
            unsafe {
                drop(Box::from_raw(listener_ptr));
            }
            let error = format!(
                "create clipboard listener window failed: {}",
                unsafe { GetLastError() }
            );
            let _ = ready_tx.send(Err(error.clone()));
            return Err(error);
        }

        if unsafe { AddClipboardFormatListener(hwnd) } == 0 {
            let error = format!(
                "register clipboard format listener failed: {}",
                unsafe { GetLastError() }
            );
            let _ = ready_tx.send(Err(error.clone()));
            return Err(error);
        }

        let _ = ready_tx.send(Ok(()));

        let mut message = unsafe { std::mem::zeroed::<MSG>() };
        loop {
            let result = unsafe { GetMessageW(&mut message, hwnd, 0, 0) };
            if result == -1 {
                let error = format!(
                    "clipboard listener message loop failed: {}",
                    unsafe { GetLastError() }
                );
                if let Some(sender) = listener_sender(hwnd) {
                    let _ = sender.send(ClipboardListenerSignal::Failed(error.clone()));
                }
                return Err(error);
            }
            if result == 0 {
                break;
            }

            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }

        unsafe {
            RemoveClipboardFormatListener(hwnd);
        }

        Ok(())
    }

    unsafe extern "system" fn listener_wndproc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_NCCREATE => {
                let create_struct = &*(lparam as *const CREATESTRUCTW);
                let sender_ptr = create_struct.lpCreateParams
                    as *mut mpsc::UnboundedSender<ClipboardListenerSignal>;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, sender_ptr as isize);
                1
            }
            WM_CLIPBOARDUPDATE => {
                if let Some(sender) = listener_sender(hwnd) {
                    let _ = sender.send(ClipboardListenerSignal::ClipboardChanged);
                }
                0
            }
            WM_NCDESTROY => {
                let sender_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA)
                    as *mut mpsc::UnboundedSender<ClipboardListenerSignal>;
                if !sender_ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(sender_ptr));
                }
                DefWindowProcW(hwnd, message, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    fn listener_sender(hwnd: HWND) -> Option<&'static mpsc::UnboundedSender<ClipboardListenerSignal>> {
        let sender_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) }
            as *const mpsc::UnboundedSender<ClipboardListenerSignal>;
        if sender_ptr.is_null() {
            None
        } else {
            Some(unsafe { &*sender_ptr })
        }
    }

    fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
