use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};

use image::GenericImageView;

use crate::clipboard::image::{format_byte_size, png_data_url, snapshot_from_png_bytes, MAX_IMAGE_BYTES};
use crate::clipboard::types::{ClipboardFileItem, ClipboardSnapshot};
use crate::storage::staging;

pub const MAX_BATCH_BYTES: u64 = 500 * 1024 * 1024;
pub const DEFAULT_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
pub const FILE_TRANSFER_LIMIT_MESSAGE: &str = "所选文件超出传输上限，同步失败";
pub const SYNC_NOT_CONNECTED_MESSAGE: &str = "当前未连接其他设备，同步失败";
const HISTORY_PREVIEW_MAX_EDGE: u32 = 480;
const MAX_HISTORY_FILE_NAMES: usize = 50;

/// User-facing validation errors from local clipboard reads (not transient I/O failures).
pub fn is_user_limit_error(message: &str) -> bool {
    message == FILE_TRANSFER_LIMIT_MESSAGE
}

pub fn hash_file(path: &Path) -> Result<[u8; 32], String> {
    let file = File::open(path).map_err(|error| format!("open file failed: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 256 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("read file failed: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(*hasher.finalize().as_bytes())
}

pub fn encode_hash(hash: &[u8; 32]) -> String {
    hex::encode(hash)
}

pub fn file_list_for_meta(files: &[ClipboardFileItem]) -> Vec<ClipboardFileItem> {
    files
        .iter()
        .map(|file| ClipboardFileItem {
            file_name: file.file_name.clone(),
            size_bytes: file.size_bytes,
            content_hash: file.content_hash,
            source_path: None,
        })
        .collect()
}

pub fn file_list_hash(files: &[ClipboardFileItem]) -> [u8; 32] {
    let mut ordered = files.to_vec();
    ordered.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    let mut hasher = blake3::Hasher::new();
    for file in ordered {
        hasher.update(file.file_name.as_bytes());
        hasher.update(&file.content_hash);
    }
    *hasher.finalize().as_bytes()
}

pub fn file_meta_hash(file_name: &str, size_bytes: u64) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(file_name.as_bytes());
    hasher.update(&size_bytes.to_le_bytes());
    *hasher.finalize().as_bytes()
}

pub fn snapshot_from_file_paths_meta(
    paths: Vec<PathBuf>,
) -> Result<ClipboardSnapshot, String> {
    if paths.is_empty() {
        return Ok(ClipboardSnapshot::Empty);
    }

    let mut files = Vec::new();

    for path in paths {
        if staging::is_under_staging(&path) {
            continue;
        }

        let metadata = std::fs::metadata(&path)
            .map_err(|error| format!("read file metadata failed: {error}"))?;
        if !metadata.is_file() {
            continue;
        }

        let size_bytes = metadata.len();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file")
            .to_string();

        files.push(ClipboardFileItem {
            file_name: file_name.clone(),
            size_bytes,
            content_hash: file_meta_hash(&file_name, size_bytes),
            source_path: Some(path),
        });
    }

    if files.is_empty() {
        return Ok(ClipboardSnapshot::Empty);
    }

    Ok(ClipboardSnapshot::FileList { files })
}

pub fn snapshot_from_file_paths(
    paths: Vec<PathBuf>,
    max_file_bytes: u64,
    max_batch_bytes: u64,
) -> Result<ClipboardSnapshot, String> {
    if paths.is_empty() {
        return Ok(ClipboardSnapshot::Empty);
    }

    let mut files = Vec::new();
    let mut batch_bytes = 0u64;

    for path in paths {
        if staging::is_under_staging(&path) {
            continue;
        }

        let metadata = std::fs::metadata(&path)
            .map_err(|error| format!("read file metadata failed: {error}"))?;
        if !metadata.is_file() {
            continue;
        }

        let size_bytes = metadata.len();
        if size_bytes > max_file_bytes {
            return Err(FILE_TRANSFER_LIMIT_MESSAGE.to_string());
        }

        batch_bytes = batch_bytes.saturating_add(size_bytes);
        if batch_bytes > max_batch_bytes {
            return Err(FILE_TRANSFER_LIMIT_MESSAGE.to_string());
        }

        let content_hash = hash_file(&path)?;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file")
            .to_string();

        files.push(ClipboardFileItem {
            file_name,
            size_bytes,
            content_hash,
            source_path: Some(path),
        });
    }

    if files.is_empty() {
        return Ok(ClipboardSnapshot::Empty);
    }

    Ok(ClipboardSnapshot::FileList { files })
}

pub fn file_list_summary(files: &[ClipboardFileItem]) -> String {
    if files.is_empty() {
        return "[文件]".to_string();
    }
    if files.len() == 1 {
        return files[0].file_name.clone();
    }
    format!("{} 等 {} 个文件", files[0].file_name, files.len())
}

/// Filename text used when file sync is disabled (sync + remote clipboard write).
pub fn file_list_as_sync_text(files: &[ClipboardFileItem]) -> Option<String> {
    let summary = file_list_summary(files);
    if summary.is_empty() || summary == "[文件]" {
        None
    } else {
        Some(summary)
    }
}

pub fn file_list_size_label(files: &[ClipboardFileItem]) -> String {
    let total: u64 = files.iter().map(|file| file.size_bytes).sum();
    format_byte_size(total as usize)
}

pub fn file_names_for_history(files: &[ClipboardFileItem]) -> Vec<String> {
    files
        .iter()
        .take(MAX_HISTORY_FILE_NAMES)
        .map(|file| file.file_name.clone())
        .collect()
}

pub fn is_image_file_name(file_name: &str) -> bool {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
    )
}

/// When the user copies a single image file in Explorer, treat it as an image snapshot for sync.
pub fn image_snapshot_from_single_file(files: &[ClipboardFileItem]) -> Option<ClipboardSnapshot> {
    if files.len() != 1 || !is_image_file_name(&files[0].file_name) {
        return None;
    }

    let file = &files[0];
    if file.size_bytes > MAX_IMAGE_BYTES as u64 {
        return None;
    }

    let path = file.source_path.as_ref()?;
    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    let rgba = image.to_rgba8();
    let mut png_bytes = Vec::new();
    rgba.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .ok()?;
    snapshot_from_png_bytes(png_bytes)
}

pub fn history_preview_for_files(files: &[ClipboardFileItem]) -> Option<String> {
    if files.len() != 1 || !is_image_file_name(&files[0].file_name) {
        return None;
    }

    let path = files[0].source_path.as_ref()?;
    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    let (width, height) = image.dimensions();
    let preview = if width <= HISTORY_PREVIEW_MAX_EDGE && height <= HISTORY_PREVIEW_MAX_EDGE {
        image
    } else {
        image.resize(
            HISTORY_PREVIEW_MAX_EDGE,
            HISTORY_PREVIEW_MAX_EDGE,
            image::imageops::FilterType::Triangle,
        )
    };
    let rgba = preview.to_rgba8();
    let mut png_bytes = Vec::new();
    rgba.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .ok()?;
    Some(png_data_url(&png_bytes))
}

pub fn history_summary_for_files(files: &[ClipboardFileItem]) -> String {
    if files.len() == 1 && is_image_file_name(&files[0].file_name) {
        if let Some(path) = files[0].source_path.as_ref() {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(image) = image::load_from_memory(&bytes) {
                    let (width, height) = image.dimensions();
                    return format!("[图片] {width}×{height}");
                }
            }
        }
    }

    file_list_summary(files)
}
