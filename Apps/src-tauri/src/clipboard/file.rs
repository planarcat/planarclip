use std::collections::BTreeMap;
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
pub const EMPTY_FOLDER_SYNC_MESSAGE: &str = "该文件夹为空，无法同步到其他设备";
const HISTORY_PREVIEW_MAX_EDGE: u32 = 480;
const MAX_HISTORY_FILE_NAMES: usize = 50;

/// User-facing validation errors from local clipboard reads (not transient I/O failures).
pub fn is_user_limit_error(message: &str) -> bool {
    message == FILE_TRANSFER_LIMIT_MESSAGE || message == EMPTY_FOLDER_SYNC_MESSAGE
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

struct CollectedPathEntry {
    relative_name: String,
    source_path: PathBuf,
    size_bytes: u64,
}

fn collect_clipboard_path_entries(paths: Vec<PathBuf>) -> Result<Vec<CollectedPathEntry>, String> {
    let mut entries = Vec::new();
    for path in paths {
        collect_single_clipboard_path(&path, &mut entries)?;
    }
    Ok(entries)
}

fn collect_single_clipboard_path(
    path: &Path,
    entries: &mut Vec<CollectedPathEntry>,
) -> Result<(), String> {
    if staging::is_under_staging(path) {
        return Ok(());
    }

    let metadata =
        std::fs::metadata(path).map_err(|error| format!("read file metadata failed: {error}"))?;
    if metadata.is_file() {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file")
            .to_string();
        entries.push(CollectedPathEntry {
            relative_name: file_name,
            source_path: path.to_path_buf(),
            size_bytes: metadata.len(),
        });
        return Ok(());
    }

    if metadata.is_dir() {
        let root_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("folder")
            .to_string();
        let mut nested = Vec::new();
        walk_directory_files(path, &root_name, &mut nested)?;
        if nested.is_empty() {
            entries.push(CollectedPathEntry {
                relative_name: root_name,
                source_path: path.to_path_buf(),
                size_bytes: 0,
            });
        } else {
            entries.extend(nested);
        }
    }

    Ok(())
}

fn walk_directory_files(
    dir: &Path,
    relative_prefix: &str,
    out: &mut Vec<CollectedPathEntry>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir)
        .map_err(|error| format!("read folder failed: {error}"))?
    {
        let entry = entry.map_err(|error| format!("read folder entry failed: {error}"))?;
        let path = entry.path();
        if staging::is_under_staging(&path) {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|error| format!("read folder entry metadata failed: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let relative_name = if relative_prefix.is_empty() {
            name.to_string()
        } else {
            format!("{relative_prefix}/{name}")
        };

        if metadata.is_dir() {
            walk_directory_files(&path, &relative_name, out)?;
        } else if metadata.is_file() {
            out.push(CollectedPathEntry {
                relative_name,
                source_path: path,
                size_bytes: metadata.len(),
            });
        }
    }
    Ok(())
}

fn is_empty_directory_entry(entry: &CollectedPathEntry) -> bool {
    entry.size_bytes == 0
        && std::fs::metadata(&entry.source_path)
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
}

pub fn snapshot_from_file_paths_meta(
    paths: Vec<PathBuf>,
) -> Result<ClipboardSnapshot, String> {
    if paths.is_empty() {
        return Ok(ClipboardSnapshot::Empty);
    }

    let collected = collect_clipboard_path_entries(paths)?;
    if collected.is_empty() {
        return Ok(ClipboardSnapshot::Empty);
    }

    let files = collected
        .into_iter()
        .map(|entry| ClipboardFileItem {
            file_name: entry.relative_name.clone(),
            size_bytes: entry.size_bytes,
            content_hash: file_meta_hash(&entry.relative_name, entry.size_bytes),
            source_path: Some(entry.source_path),
        })
        .collect();

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

    let collected = collect_clipboard_path_entries(paths)?;
    if collected.is_empty() {
        return Ok(ClipboardSnapshot::Empty);
    }

    if collected.len() == 1 && is_empty_directory_entry(&collected[0]) {
        return Err(EMPTY_FOLDER_SYNC_MESSAGE.to_string());
    }

    let mut files = Vec::new();
    let mut batch_bytes = 0u64;

    for entry in collected {
        if is_empty_directory_entry(&entry) {
            continue;
        }

        if entry.size_bytes > max_file_bytes {
            return Err(FILE_TRANSFER_LIMIT_MESSAGE.to_string());
        }

        batch_bytes = batch_bytes.saturating_add(entry.size_bytes);
        if batch_bytes > max_batch_bytes {
            return Err(FILE_TRANSFER_LIMIT_MESSAGE.to_string());
        }

        let content_hash = hash_file(&entry.source_path)?;
        files.push(ClipboardFileItem {
            file_name: entry.relative_name,
            size_bytes: entry.size_bytes,
            content_hash,
            source_path: Some(entry.source_path),
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
    if let Some(root) = common_folder_root_label(files) {
        if files.len() == 1 && files[0].size_bytes == 0 {
            return root;
        }
        return format!("{root}（{} 个文件）", files.len());
    }
    if files.len() == 1 {
        return files[0].file_name.clone();
    }
    format!("{} 等 {} 个文件", files[0].file_name, files.len())
}

pub fn file_list_needs_batch_transfer(files: &[ClipboardFileItem]) -> bool {
    files.len() > 1
        || files.iter().any(|file| {
            file.file_name.contains('/') || file.file_name.contains('\\')
        })
}

/// Build CF_HDROP / file-URL roots so paste keeps Explorer-style top-level items (files + folders).
pub fn clipboard_hdrop_paths(batch_dir: &Path, received: &[PathBuf]) -> Vec<PathBuf> {
    if received.is_empty() {
        return Vec::new();
    }
    if let Some(root) = single_folder_hdrop_root(batch_dir, received) {
        return vec![root];
    }
    hdrop_roots_by_top_level(batch_dir, received)
}

/// Group staged paths by first path segment; nested groups paste as a folder, not loose files.
fn hdrop_roots_by_top_level(batch_dir: &Path, received: &[PathBuf]) -> Vec<PathBuf> {
    let mut groups: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for path in received {
        let relative = match path.strip_prefix(batch_dir) {
            Ok(value) => value,
            Err(_) => return received.to_vec(),
        };
        let mut components = relative.components();
        let first = match components.next() {
            Some(component) => component.as_os_str().to_string_lossy().to_string(),
            None => continue,
        };
        groups.entry(first).or_default().push(path.clone());
    }

    let mut roots = Vec::with_capacity(groups.len());
    for (first_name, group_paths) in groups {
        let candidate_dir = batch_dir.join(&first_name);
        let single_top_level_file = group_paths.len() == 1
            && group_paths[0]
                .strip_prefix(batch_dir)
                .map(|relative| relative.components().count() == 1)
                .unwrap_or(false)
            && candidate_dir.is_file();
        let nested_under_name = group_paths.iter().any(|path| {
            path.strip_prefix(batch_dir)
                .map(|relative| relative.components().count() > 1)
                .unwrap_or(false)
        });

        if single_top_level_file {
            roots.push(group_paths[0].clone());
        } else if nested_under_name && candidate_dir.is_dir() {
            roots.push(candidate_dir);
        } else if candidate_dir.is_dir() {
            roots.push(candidate_dir);
        } else {
            roots.extend(group_paths);
        }
    }
    roots
}

fn common_folder_root_label(files: &[ClipboardFileItem]) -> Option<String> {
    let first = files.first()?;
    let separator = path_separator_in_name(&first.file_name)?;
    let root = first.file_name.split(separator).next()?;
    if root.is_empty() {
        return None;
    }
    let prefix = format!("{root}{separator}");
    let all_under_root = files.iter().all(|file| {
        file.file_name == root || file.file_name.starts_with(&prefix)
    });
    if all_under_root {
        Some(root.to_string())
    } else {
        None
    }
}

fn path_separator_in_name(name: &str) -> Option<char> {
    if name.contains('/') {
        Some('/')
    } else if name.contains('\\') {
        Some('\\')
    } else {
        None
    }
}

pub(crate) fn single_folder_hdrop_root(batch_dir: &Path, received: &[PathBuf]) -> Option<PathBuf> {
    if received.is_empty() {
        return None;
    }

    let mut root_component: Option<String> = None;
    for path in received {
        let relative = path.strip_prefix(batch_dir).ok()?;
        let mut components = relative.components();
        let first = components.next()?;
        let first_name = first.as_os_str().to_string_lossy().to_string();
        if components.next().is_none() {
            return None;
        }
        match &root_component {
            None => root_component = Some(first_name),
            Some(expected) if expected == &first_name => {}
            _ => return None,
        }
    }

    let root_name = root_component?;
    let root_path = batch_dir.join(&root_name);
    if root_path.is_dir() {
        Some(root_path)
    } else {
        None
    }
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

pub fn file_items_from_received_paths(
    batch_dir: Option<&Path>,
    received_paths: &[PathBuf],
) -> Vec<ClipboardFileItem> {
    received_paths
        .iter()
        .filter_map(|path| {
            let metadata = std::fs::metadata(path).ok()?;
            if !metadata.is_file() {
                return None;
            }
            let file_name = if let Some(batch_dir) = batch_dir {
                path.strip_prefix(batch_dir)
                    .ok()
                    .and_then(|relative| relative.to_str())
                    .map(|value| value.replace('\\', "/"))?
            } else {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .map(str::to_string)?
            };
            let content_hash = hash_file(path).ok()?;
            Some(ClipboardFileItem {
                file_name,
                size_bytes: metadata.len(),
                content_hash,
                source_path: Some(path.clone()),
            })
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn common_folder_root_label_detects_shared_prefix() {
        let files = vec![
            ClipboardFileItem {
                file_name: "Photos/a.jpg".to_string(),
                size_bytes: 1,
                content_hash: [0; 32],
                source_path: None,
            },
            ClipboardFileItem {
                file_name: "Photos/b.jpg".to_string(),
                size_bytes: 1,
                content_hash: [0; 32],
                source_path: None,
            },
        ];
        assert_eq!(
            common_folder_root_label(&files).as_deref(),
            Some("Photos")
        );
        assert_eq!(file_list_summary(&files), "Photos（2 个文件）");
    }

    #[test]
    fn clipboard_hdrop_paths_uses_folder_root_for_nested_batch() {
        let batch_dir = std::env::temp_dir().join(format!(
            "planarclip-folder-hdrop-{}",
            uuid::Uuid::new_v4()
        ));
        let folder = batch_dir.join("MyFolder");
        let nested = folder.join("nested");
        fs::create_dir_all(&nested).unwrap();
        let file_path = nested.join("note.txt");
        fs::write(&file_path, b"hello").unwrap();

        let received = vec![file_path];
        let hdrop = clipboard_hdrop_paths(&batch_dir, &received);
        assert_eq!(hdrop, vec![folder]);

        let _ = fs::remove_dir_all(&batch_dir);
    }

    #[test]
    fn clipboard_hdrop_paths_preserves_folders_in_mixed_batch() {
        let batch_dir = std::env::temp_dir().join(format!(
            "planarclip-mixed-hdrop-{}",
            uuid::Uuid::new_v4()
        ));
        let scripts = batch_dir.join("scripts");
        fs::create_dir_all(scripts.clone()).unwrap();
        fs::write(scripts.join("a.mjs"), b"a").unwrap();
        fs::write(scripts.join("b.mjs"), b"b").unwrap();
        fs::write(batch_dir.join("AGENTS.md"), b"#").unwrap();
        fs::write(batch_dir.join("package.json"), b"{}").unwrap();

        let received = vec![
            batch_dir.join("AGENTS.md"),
            batch_dir.join("package.json"),
            scripts.join("a.mjs"),
            scripts.join("b.mjs"),
        ];
        let hdrop = clipboard_hdrop_paths(&batch_dir, &received);
        assert_eq!(
            hdrop,
            vec![
                batch_dir.join("AGENTS.md"),
                batch_dir.join("package.json"),
                scripts,
            ]
        );

        let _ = fs::remove_dir_all(&batch_dir);
    }

    #[test]
    fn collect_clipboard_path_entries_expands_directory() {
        let root = std::env::temp_dir().join(format!(
            "planarclip-folder-collect-{}",
            uuid::Uuid::new_v4()
        ));
        let folder = root.join("Bundle");
        fs::create_dir_all(folder.join("inner")).unwrap();
        let file_path = folder.join("inner").join("data.bin");
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"x").unwrap();

        let entries = collect_clipboard_path_entries(vec![folder.clone()]).expect("collect");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relative_name, "Bundle/inner/data.bin");
        assert_eq!(entries[0].source_path, file_path);

        let _ = fs::remove_dir_all(&root);
    }
}
