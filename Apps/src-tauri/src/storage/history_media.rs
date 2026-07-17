use std::fs;
use std::path::{Path, PathBuf};

use crate::storage::json;

const MEDIA_DIR_NAME: &str = "history_media";

pub fn media_root() -> PathBuf {
    json::config_path()
        .parent()
        .map(|parent| parent.join(MEDIA_DIR_NAME))
        .unwrap_or_else(|| PathBuf::from(MEDIA_DIR_NAME))
}

pub fn ensure_media_dir() -> Result<PathBuf, String> {
    let root = media_root();
    fs::create_dir_all(&root).map_err(|error| format!("create history media dir failed: {error}"))?;
    Ok(root)
}

pub fn relative_ref(entry_id: &str) -> String {
    format!("{MEDIA_DIR_NAME}/{}.png", sanitize_entry_id(entry_id))
}

pub fn write_media(entry_id: &str, png_bytes: &[u8]) -> Result<String, String> {
    ensure_media_dir()?;
    let relative = relative_ref(entry_id);
    let absolute = absolute_path(&relative)?;
    fs::write(&absolute, png_bytes)
        .map_err(|error| format!("write history media failed: {error}"))?;
    Ok(relative)
}

pub fn absolute_path(relative_ref: &str) -> Result<PathBuf, String> {
    let normalized = relative_ref.replace('\\', "/");
    if !normalized.starts_with(&format!("{MEDIA_DIR_NAME}/"))
        || normalized.contains("..")
        || !normalized.ends_with(".png")
    {
        return Err("invalid history media reference".to_string());
    }

    let file_name = normalized
        .strip_prefix(&format!("{MEDIA_DIR_NAME}/"))
        .ok_or_else(|| "invalid history media reference".to_string())?;
    Ok(media_root().join(file_name))
}

pub fn delete_by_ref(relative_ref: &str) -> Result<(), String> {
    let path = absolute_path(relative_ref)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("delete history media failed: {error}"))?;
    }
    Ok(())
}

pub fn clear_all() -> Result<(), String> {
    let root = media_root();
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&root).map_err(|error| format!("read history media dir failed: {error}"))? {
        let entry = entry.map_err(|error| format!("read history media entry failed: {error}"))?;
        let path = entry.path();
        if path.is_file() {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

pub fn resolve_data_url(relative_ref: &str) -> Result<String, String> {
    let path = absolute_path(relative_ref)?;
    if !path.is_file() {
        return Err("history media not found".to_string());
    }
    let bytes = fs::read(&path).map_err(|error| format!("read history media failed: {error}"))?;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    Ok(format!("data:image/png;base64,{}", BASE64.encode(&bytes)))
}

/// Delete media files not referenced by any of the given refs (startup GC).
pub fn gc_orphans(refs: &[String]) {
    let root = media_root();
    let Ok(entries) = fs::read_dir(&root) else {
        return;
    };
    let keep: std::collections::HashSet<&str> = refs.iter().map(|s| s.as_str()).collect();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let Some(name_str) = file_name.to_str() else {
            continue;
        };
        let this_ref = format!("{MEDIA_DIR_NAME}/{name_str}");
        if !keep.contains(this_ref.as_str()) {
            let _ = fs::remove_file(&path);
        }
    }
}

/// Migrate legacy `history_thumbs/` and `history_images/` files into `history_media/`,
/// returning the updated refs (old prefix -> "history_media/").
pub fn migrate_legacy_dir(parent: &Path) -> usize {
    let dest = parent.join(MEDIA_DIR_NAME);
    let _ = fs::create_dir_all(&dest);
    let mut moved = 0;
    for legacy in ["history_thumbs", "history_images"] {
        let src = parent.join(legacy);
        let Ok(entries) = fs::read_dir(&src) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(name) = path.file_name() {
                let target = dest.join(name);
                if fs::rename(&path, &target).is_ok() {
                    moved += 1;
                }
            }
        }
        let _ = fs::remove_dir(&src);
    }
    moved
}

fn sanitize_entry_id(entry_id: &str) -> String {
    entry_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        assert!(absolute_path("history_media/../secret.png").is_err());
        assert!(absolute_path("other/foo.png").is_err());
    }
}
