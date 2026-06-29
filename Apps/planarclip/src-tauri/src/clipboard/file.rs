use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::clipboard::image::format_byte_size;
use crate::clipboard::types::{ClipboardFileItem, ClipboardSnapshot};
use crate::storage::staging;

pub const MAX_BATCH_BYTES: u64 = 500 * 1024 * 1024;
pub const DEFAULT_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

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
            return Err(format!(
                "文件 {} 超过 {}，未加入同步。",
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("文件"),
                format_byte_size(max_file_bytes as usize)
            ));
        }

        batch_bytes = batch_bytes.saturating_add(size_bytes);
        if batch_bytes > max_batch_bytes {
            return Err(format!(
                "本次复制的文件总量超过 {}，未加入同步。",
                format_byte_size(max_batch_bytes as usize)
            ));
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

pub fn file_list_size_label(files: &[ClipboardFileItem]) -> String {
    let total: u64 = files.iter().map(|file| file.size_bytes).sum();
    format_byte_size(total as usize)
}
