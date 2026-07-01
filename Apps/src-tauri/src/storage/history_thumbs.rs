use std::fs;
use std::path::PathBuf;

use crate::storage::json;

const THUMBS_DIR_NAME: &str = "history_thumbs";

pub fn thumbs_root() -> PathBuf {
    json::config_path()
        .parent()
        .map(|parent| parent.join(THUMBS_DIR_NAME))
        .unwrap_or_else(|| PathBuf::from(THUMBS_DIR_NAME))
}

pub fn ensure_thumbs_dir() -> Result<PathBuf, String> {
    let root = thumbs_root();
    fs::create_dir_all(&root).map_err(|error| format!("create history thumbs dir failed: {error}"))?;
    Ok(root)
}

pub fn relative_ref(entry_id: &str) -> String {
    format!("{THUMBS_DIR_NAME}/{}.png", sanitize_entry_id(entry_id))
}

pub fn write_png(entry_id: &str, png_bytes: &[u8]) -> Result<String, String> {
    ensure_thumbs_dir()?;
    let relative = relative_ref(entry_id);
    let absolute = absolute_path(&relative)?;
    fs::write(&absolute, png_bytes)
        .map_err(|error| format!("write history thumb failed: {error}"))?;
    Ok(relative)
}

pub fn absolute_path(relative_ref: &str) -> Result<PathBuf, String> {
    let normalized = relative_ref.replace('\\', "/");
    if !normalized.starts_with(&format!("{THUMBS_DIR_NAME}/"))
        || normalized.contains("..")
        || !normalized.ends_with(".png")
    {
        return Err("invalid history thumbnail reference".to_string());
    }

    let file_name = normalized
        .strip_prefix(&format!("{THUMBS_DIR_NAME}/"))
        .ok_or_else(|| "invalid history thumbnail reference".to_string())?;

    Ok(thumbs_root().join(file_name))
}

pub fn delete_by_ref(relative_ref: &str) -> Result<(), String> {
    let path = absolute_path(relative_ref)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("delete history thumb failed: {error}"))?;
    }
    Ok(())
}

pub fn clear_all() -> Result<(), String> {
    let root = thumbs_root();
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&root).map_err(|error| format!("read history thumbs dir failed: {error}"))? {
        let entry = entry.map_err(|error| format!("read history thumb entry failed: {error}"))?;
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
        return Err("history thumbnail not found".to_string());
    }
    let bytes = fs::read(&path).map_err(|error| format!("read history thumb failed: {error}"))?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
    ))
}

pub fn resolve_absolute(relative_ref: &str) -> Result<String, String> {
    let path = absolute_path(relative_ref)?;
    if !path.is_file() {
        return Err("history thumbnail not found".to_string());
    }
    path.canonicalize()
        .map(|value| value.to_string_lossy().to_string())
        .map_err(|error| format!("resolve history thumb failed: {error}"))
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
        assert!(absolute_path("history_thumbs/../secret.png").is_err());
        assert!(absolute_path("other/foo.png").is_err());
    }
}
