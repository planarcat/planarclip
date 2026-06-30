//! Cross-platform file clipboard helpers (Windows CF_HDROP, macOS NSPasteboard URLs).

#[cfg(not(any(windows, target_os = "macos")))]
use std::path::PathBuf;

#[cfg(windows)]
pub use crate::platform::windows::clipboard::{
    current_sequence, has_file_format, read_file_paths, write_file_paths,
};

#[cfg(target_os = "macos")]
pub use crate::platform::macos::clipboard::{
    current_sequence, has_file_format, read_file_paths, write_file_paths,
};

#[cfg(not(any(windows, target_os = "macos")))]
pub fn current_sequence() -> Option<u32> {
    None
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn has_file_format() -> bool {
    false
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn read_file_paths() -> Option<Vec<PathBuf>> {
    None
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn write_file_paths(_paths: &[PathBuf]) -> Result<(), String> {
    Err("clipboard file write is not supported on this platform yet".into())
}
