use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::storage::json;

const STAGING_DIR_NAME: &str = "staging";
const GC_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

pub fn staging_root() -> PathBuf {
    json::config_path()
        .parent()
        .map(|parent| parent.join(STAGING_DIR_NAME))
        .unwrap_or_else(|| PathBuf::from(STAGING_DIR_NAME))
}

pub fn ensure_staging() -> Result<PathBuf, String> {
    let root = staging_root();
    fs::create_dir_all(&root).map_err(|error| format!("create staging dir failed: {error}"))?;
    Ok(root)
}

pub fn batch_dir(batch_id: &str) -> PathBuf {
    staging_root().join(sanitize_path_component(batch_id))
}

pub fn temp_transfer_path(transfer_id: &str) -> PathBuf {
    staging_root().join(format!("{}.part", sanitize_path_component(transfer_id)))
}

pub fn image_sync_path(content_hash: &[u8; 32]) -> PathBuf {
    staging_root().join(format!(
        "planarclip-sync-{}.png",
        hex::encode(&content_hash[..8])
    ))
}

pub fn resolve_unique_name(dir: &Path, file_name: &str) -> String {
    let safe_name = sanitize_file_name(file_name);
    let mut candidate = safe_name.clone();
    let stem = Path::new(&safe_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let extension = Path::new(&safe_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();

    let mut counter = 1u32;
    while dir.join(&candidate).exists() {
        candidate = format!("{stem} ({counter}){extension}");
        counter += 1;
    }
    candidate
}

pub fn finalize_staged_file(
    temp_path: &Path,
    batch_id: Option<&str>,
    file_name: &str,
) -> Result<PathBuf, String> {
    ensure_staging()?;
    let target_dir = match batch_id {
        Some(batch_id) => {
            let dir = batch_dir(batch_id);
            fs::create_dir_all(&dir)
                .map_err(|error| format!("create batch dir failed: {error}"))?;
            dir
        }
        None => staging_root(),
    };
    let relative = sanitize_relative_path(Path::new(file_name));
    let final_path = if relative.components().count() > 1 {
        let parent = relative
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(""));
        let leaf = relative
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        let parent_dir = target_dir.join(parent);
        fs::create_dir_all(&parent_dir)
            .map_err(|error| format!("create nested staging dir failed: {error}"))?;
        let unique_leaf = resolve_unique_name(&parent_dir, leaf);
        parent_dir.join(unique_leaf)
    } else {
        let leaf = relative
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        target_dir.join(resolve_unique_name(&target_dir, leaf))
    };
    fs::rename(temp_path, &final_path)
        .map_err(|error| format!("move staged file failed: {error}"))?;
    Ok(final_path)
}

pub fn write_png_if_absent(path: &Path, png_bytes: &[u8]) -> Result<(), String> {
    ensure_staging()?;
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create png parent dir failed: {error}"))?;
    }
    let mut file = fs::File::create(path).map_err(|error| format!("create png failed: {error}"))?;
    file.write_all(png_bytes)
        .map_err(|error| format!("write png failed: {error}"))
}

pub fn gc_staging() -> Result<usize, String> {
    let root = staging_root();
    if !root.exists() {
        return Ok(0);
    }

    let now = SystemTime::now();
    let mut removed = 0usize;
    for entry in fs::read_dir(&root).map_err(|error| format!("read staging dir failed: {error}"))? {
        let entry = entry.map_err(|error| format!("read staging entry failed: {error}"))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("read staging metadata failed: {error}"))?;
        let modified = metadata.modified().unwrap_or(now);
        if now.duration_since(modified).unwrap_or(Duration::ZERO) <= GC_MAX_AGE {
            continue;
        }
        if metadata.is_dir() {
            if fs::remove_dir_all(&path).is_ok() {
                removed += 1;
            }
        } else if fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn is_under_staging(path: &Path) -> bool {
    let root = staging_root();
    path.canonicalize()
        .ok()
        .and_then(|canonical| {
            root.canonicalize()
                .ok()
                .map(|staging_root| canonical.starts_with(staging_root))
        })
        .unwrap_or(false)
}

fn sanitize_path_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn sanitize_file_name(name: &str) -> String {
    let trimmed = name.trim();
    let file_name = Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    sanitize_path_segment(file_name)
}

fn sanitize_relative_path(path: &Path) -> PathBuf {
    let mut cleaned = PathBuf::new();
    for component in path.components() {
        if let std::path::Component::Normal(segment) = component {
            let safe = sanitize_path_segment(&segment.to_string_lossy());
            if !safe.is_empty() {
                cleaned.push(safe);
            }
        }
    }
    if cleaned.as_os_str().is_empty() {
        PathBuf::from("file")
    } else {
        cleaned
    }
}

fn sanitize_path_segment(segment: &str) -> String {
    let sanitized: String = segment
        .chars()
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                ch
            }
        })
        .collect();
    if sanitized.is_empty() {
        "file".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_unique_name_appends_suffix_on_conflict() {
        let dir = std::env::temp_dir().join(format!(
            "planarclip-staging-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("report.pdf"), b"x").unwrap();

        let resolved = resolve_unique_name(&dir, "report.pdf");
        assert_eq!(resolved, "report (1).pdf");

        let _ = fs::remove_dir_all(dir);
    }
}
