use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use tokio::sync::{broadcast, mpsc};

use crate::clipboard::types::{debug_report, ClipboardEvent, ClipboardSnapshot};

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
}

impl ClipboardMonitor {
    pub fn new(tx: broadcast::Sender<ClipboardEvent>) -> Self {
        Self {
            tx,
            last_hash: [0u8; 32],
            last_read_error: None,
        }
    }

    pub fn set_self_writing(flag: bool) {
        SELF_WRITING.store(flag, Ordering::SeqCst);
    }

    pub fn is_self_writing() -> bool {
        SELF_WRITING.load(Ordering::SeqCst)
    }

    pub async fn run(&mut self) {
        #[cfg(target_os = "windows")]
        {
            match self.run_windows_listener().await {
                Ok(()) => return,
                Err(error) => {
                    debug_report(
                        "A",
                        "clipboard/monitor.rs:44",
                        "[DEBUG] clipboard listener fallback to polling",
                        serde_json::json!({
                            "poll_interval_ms": CLIPBOARD_POLL_INTERVAL_MS,
                            "error": error,
                        }),
                    );
                }
            }
        }

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

        match Self::read_clipboard() {
            Ok(snapshot) => {
                if self.last_read_error.take().is_some() {
                    debug_report(
                        "A",
                        "clipboard/monitor.rs:70",
                        "[DEBUG] clipboard read recovered",
                        serde_json::json!({
                            "poll_interval_ms": CLIPBOARD_POLL_INTERVAL_MS,
                        }),
                    );
                }

                let hash = snapshot.content_hash();
                if Self::should_suppress_local_emit(hash) {
                    self.last_hash = hash;
                    debug_report(
                        "A",
                        "clipboard/monitor.rs:81",
                        "[DEBUG] monitor suppressed local clipboard echo",
                        serde_json::json!({
                            "hash": hex::encode(hash),
                            "text_len": snapshot.text().map(|text| text.len()).unwrap_or(0),
                        }),
                    );
                    return;
                }

                if hash != self.last_hash {
                    self.last_hash = hash;
                    debug_report(
                        "A",
                        "clipboard/monitor.rs:94",
                        "[DEBUG] monitor emitted local clipboard event",
                        serde_json::json!({
                            "hash": hex::encode(hash),
                            "self_writing": Self::is_self_writing(),
                            "text_len": snapshot.text().map(|text| text.len()).unwrap_or(0),
                        }),
                    );
                    let _ = self.tx.send(ClipboardEvent::local(snapshot));
                }
            }
            Err(error) => {
                if self.last_read_error.as_deref() != Some(error.as_str()) {
                    self.last_read_error = Some(error.clone());
                    debug_report(
                        "A",
                        "clipboard/monitor.rs:108",
                        "[DEBUG] clipboard read failed",
                        serde_json::json!({
                            "poll_interval_ms": CLIPBOARD_POLL_INTERVAL_MS,
                            "error": error,
                        }),
                    );
                }
            }
        }
    }

    fn read_clipboard() -> Result<ClipboardSnapshot, String> {
        let mut clipboard = arboard::Clipboard::new().map_err(|error| format!("clipboard init failed: {error}"))?;
        let text = clipboard
            .get_text()
            .map_err(|error| format!("clipboard read failed: {error}"))?;

        Ok(ClipboardSnapshot::Text(text))
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

    pub fn write_clipboard(text: &str) {
        let hash = *blake3::hash(text.as_bytes()).as_bytes();
        debug_report(
            "A",
            "clipboard/monitor.rs:143",
            "[DEBUG] write_clipboard started",
            serde_json::json!({
                "hash": hex::encode(hash),
                "text_len": text.len(),
            }),
        );
        if let Ok(mut state) = SUPPRESSED_REMOTE_WRITE.lock() {
            *state = Some(SuppressedRemoteWrite {
                hash,
                until_ms: now_ms() + REMOTE_WRITE_SUPPRESSION_MS,
            });
        }
        Self::set_self_writing(true);
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
        Self::set_self_writing(false);
        debug_report(
            "A",
            "clipboard/monitor.rs:159",
            "[DEBUG] write_clipboard finished",
            serde_json::json!({
                "hash": hex::encode(hash),
                "self_writing": Self::is_self_writing(),
                "suppression_until_ms": now_ms() + REMOTE_WRITE_SUPPRESSION_MS,
            }),
        );
    }

    #[cfg(target_os = "windows")]
    async fn run_windows_listener(&mut self) -> Result<(), String> {
        let (signal_tx, mut signal_rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);

        std::thread::spawn(move || windows_listener::run(signal_tx, ready_tx));

        match ready_rx.recv() {
            Ok(Ok(())) => {
                debug_report(
                    "A",
                    "clipboard/monitor.rs:178",
                    "[DEBUG] clipboard listener started",
                    serde_json::json!({
                        "mode": "windows-event",
                    }),
                );
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => return Err("clipboard listener startup channel closed unexpectedly".to_string()),
        }

        while let Some(signal) = signal_rx.recv().await {
            match signal {
                ClipboardListenerSignal::ClipboardChanged => self.capture_clipboard_change(),
                ClipboardListenerSignal::Failed(error) => return Err(error),
            }
        }

        Err("clipboard listener exited unexpectedly".to_string())
    }
}

#[cfg(target_os = "windows")]
enum ClipboardListenerSignal {
    ClipboardChanged,
    Failed(String),
}

#[cfg(target_os = "windows")]
mod windows_listener {
    use std::ffi::c_void;

    use tokio::sync::mpsc;
    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::DataExchange::{AddClipboardFormatListener, RemoveClipboardFormatListener};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW, RegisterClassW,
        SetWindowLongPtrW, TranslateMessage, CREATESTRUCTW, GWLP_USERDATA, HWND_MESSAGE, MSG, WM_CLIPBOARDUPDATE,
        WM_NCCREATE, WM_NCDESTROY, WNDCLASSW,
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

        // 用消息窗口接收系统剪贴板变更通知，避免继续依赖高频轮询。
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
            let error = format!("create clipboard listener window failed: {}", unsafe { GetLastError() });
            let _ = ready_tx.send(Err(error.clone()));
            return Err(error);
        }

        if unsafe { AddClipboardFormatListener(hwnd) } == 0 {
            let error = format!("register clipboard format listener failed: {}", unsafe { GetLastError() });
            let _ = ready_tx.send(Err(error.clone()));
            return Err(error);
        }

        let _ = ready_tx.send(Ok(()));

        let mut message = unsafe { std::mem::zeroed::<MSG>() };
        loop {
            let result = unsafe { GetMessageW(&mut message, hwnd, 0, 0) };
            if result == -1 {
                let error = format!("clipboard listener message loop failed: {}", unsafe { GetLastError() });
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
                let sender_ptr = create_struct.lpCreateParams as *mut mpsc::UnboundedSender<ClipboardListenerSignal>;
                // 将 sender 指针挂到窗口用户数据，供系统回调把事件回送到 Rust 通道。
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
                let sender_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut mpsc::UnboundedSender<ClipboardListenerSignal>;
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
        let sender_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const mpsc::UnboundedSender<ClipboardListenerSignal>;
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
