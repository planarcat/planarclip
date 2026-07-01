use std::fs;
use std::path::{Path, PathBuf};

use crate::clipboard::file::single_folder_hdrop_root;
use crate::storage::staging;

/// Resolved save location for inbound synced files (custom path or system Downloads).
pub fn resolve_sync_files_save_dir(custom: Option<&str>) -> Result<PathBuf, String> {
    if let Some(value) = custom {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if !path.is_absolute() {
                return Err("保存路径必须是完整路径，请重新选择文件夹。".to_string());
            }
            if path.exists() && !path.is_dir() {
                return Err("保存路径不是文件夹，请重新选择。".to_string());
            }
            fs::create_dir_all(&path)
                .map_err(|_| "无法创建保存文件夹，请检查路径权限后重试。".to_string())?;
            return Ok(path);
        }
    }

    default_downloads_dir()
}

pub fn default_downloads_dir() -> Result<PathBuf, String> {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
        .ok_or_else(|| "无法定位系统下载文件夹，请在设置中手动选择保存路径。".to_string())
        .and_then(|path| {
            fs::create_dir_all(&path)
                .map_err(|_| "无法创建下载文件夹，请检查路径权限后重试。".to_string())?;
            Ok(path)
        })
}

pub struct ExportedSyncFiles {
    pub clipboard_paths: Vec<PathBuf>,
    pub file_paths: Vec<PathBuf>,
}

/// Copy received staging files into the configured save directory for long-term storage.
pub fn export_received_files_to_save_dir(
    batch_dir: Option<&Path>,
    received_paths: &[PathBuf],
    save_root: &Path,
) -> Result<ExportedSyncFiles, String> {
    if received_paths.is_empty() {
        return Ok(ExportedSyncFiles {
            clipboard_paths: Vec::new(),
            file_paths: Vec::new(),
        });
    }

    fs::create_dir_all(save_root)
        .map_err(|_| "无法写入保存文件夹，请检查路径权限后重试。".to_string())?;

    if let Some(batch_dir) = batch_dir {
        if let Some(folder_root) = single_folder_hdrop_root(batch_dir, received_paths) {
            let folder_name = folder_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("folder");
            let unique_name = staging::resolve_unique_name(save_root, folder_name);
            let dest = save_root.join(unique_name);
            copy_dir_recursive(&folder_root, &dest)?;
            let file_paths = collect_file_paths(&dest)?;
            return Ok(ExportedSyncFiles {
                clipboard_paths: vec![dest],
                file_paths,
            });
        }

        let mut file_paths = Vec::with_capacity(received_paths.len());
        for source in received_paths {
            let relative = source
                .strip_prefix(batch_dir)
                .map_err(|_| "接收文件路径异常，未能保存到本地。".to_string())?;
            let dest = unique_path_under(save_root, relative)?;
            if source.is_dir() {
                copy_dir_recursive(source, &dest)?;
            } else if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|_| "无法创建保存文件夹，请检查路径权限后重试。".to_string())?;
                fs::copy(source, &dest)
                    .map_err(|_| "保存同步文件失败，请检查磁盘空间或路径权限。".to_string())?;
            }
            file_paths.push(dest);
        }

        return Ok(ExportedSyncFiles {
            clipboard_paths: file_paths.clone(),
            file_paths,
        });
    }

    let mut file_paths = Vec::with_capacity(received_paths.len());
    for source in received_paths {
        let leaf = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        let dest = save_root.join(staging::resolve_unique_name(save_root, leaf));
        if source.is_dir() {
            copy_dir_recursive(source, &dest)?;
        } else {
            fs::copy(source, &dest)
                .map_err(|_| "保存同步文件失败，请检查磁盘空间或路径权限。".to_string())?;
        }
        file_paths.push(dest);
    }

    Ok(ExportedSyncFiles {
        clipboard_paths: file_paths.clone(),
        file_paths,
    })
}

fn unique_path_under(base: &Path, relative: &Path) -> Result<PathBuf, String> {
    let parent = relative.parent().filter(|parent| !parent.as_os_str().is_empty());
    let leaf = relative
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let target_parent = match parent {
        Some(parent) => base.join(parent),
        None => base.to_path_buf(),
    };
    fs::create_dir_all(&target_parent)
        .map_err(|_| "无法创建保存文件夹，请检查路径权限后重试。".to_string())?;
    Ok(target_parent.join(staging::resolve_unique_name(&target_parent, leaf)))
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest)
        .map_err(|_| "无法创建保存文件夹，请检查路径权限后重试。".to_string())?;
    for entry in fs::read_dir(source)
        .map_err(|_| "读取同步文件夹失败，未能完整保存。".to_string())?
    {
        let entry = entry.map_err(|_| "读取同步文件夹失败，未能完整保存。".to_string())?;
        let file_type = entry
            .file_type()
            .map_err(|_| "读取同步文件夹失败，未能完整保存。".to_string())?;
        let target = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)
                .map_err(|_| "保存同步文件失败，请检查磁盘空间或路径权限。".to_string())?;
        }
    }
    Ok(())
}

fn collect_file_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    let mut paths = Vec::new();
    collect_file_paths_inner(root, &mut paths)?;
    Ok(paths)
}

fn collect_file_paths_inner(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|_| "读取同步文件夹失败，未能完整保存。".to_string())?
    {
        let entry = entry.map_err(|_| "读取同步文件夹失败，未能完整保存。".to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_file_paths_inner(&path, paths)?;
        } else {
            paths.push(path);
        }
    }
    Ok(())
}
