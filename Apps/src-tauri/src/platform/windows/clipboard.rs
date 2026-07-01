use std::io::Cursor;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use image::RgbaImage;
use clipboard_win::formats::{CF_DIB, CF_DIBV5, CF_HDROP};
use clipboard_win::{is_format_avail, register_format, Clipboard};

use crate::clipboard::image::{decode_png_to_rgba, png_from_dib, snapshot_from_png_bytes};
use crate::clipboard::types::ClipboardSnapshot;
use crate::storage::staging;

/// Returns the current clipboard sequence number without opening the clipboard.
pub fn current_sequence() -> Option<u32> {
    clipboard_win::seq_num().map(|seq| seq.get())
}

/// Reads file paths from the Windows clipboard when CF_HDROP is present.
pub fn read_file_paths() -> Option<Vec<PathBuf>> {
    if !is_format_avail(CF_HDROP) {
        return None;
    }

    let _clipboard = Clipboard::new_attempts(10).ok()?;
    let hdrop = read_format_bytes(CF_HDROP)?;
    parse_hdrop_paths(&hdrop)
}

pub fn has_file_format() -> bool {
    is_format_avail(CF_HDROP)
}

/// Reads image data from the Windows clipboard, including formats that arboard skips (e.g. CF_DIB).
pub fn read_snapshot() -> Option<ClipboardSnapshot> {
    if !has_image_format() {
        return None;
    }

    let _clipboard = Clipboard::new_attempts(10).ok()?;

    read_png_snapshot()
        .or_else(read_dibv5_snapshot)
        .or_else(read_dib_snapshot)
}

/// True when the clipboard advertises image formats but content may not be readable yet.
pub fn has_image_format() -> bool {
    if is_format_avail(CF_DIB) || is_format_avail(CF_DIBV5) {
        return true;
    }

    register_format("PNG")
        .map(|format| is_format_avail(format.get()))
        .unwrap_or(false)
}

fn read_format_bytes(format: u32) -> Option<Vec<u8>> {
    if !is_format_avail(format) {
        return None;
    }

    let mut data = Vec::new();
    clipboard_win::raw::get_vec(format, &mut data).ok()?;
    if data.is_empty() {
        return None;
    }
    Some(data)
}

fn read_png_snapshot() -> Option<ClipboardSnapshot> {
    let format = register_format("PNG")?;
    let png_bytes = read_format_bytes(format.get())?;
    let snapshot = snapshot_from_png_bytes(png_bytes)?;
    tracing::debug!("read clipboard image from PNG format");
    Some(snapshot)
}

fn read_dibv5_snapshot() -> Option<ClipboardSnapshot> {
    let dib = read_format_bytes(CF_DIBV5)?;
    match png_from_dib(&dib) {
        Ok((png_bytes, width, height)) => {
            let snapshot = snapshot_from_png_bytes(png_bytes)?;
            tracing::debug!("read clipboard image from CF_DIBV5 ({width}x{height})");
            Some(snapshot)
        }
        Err(error) => {
            tracing::warn!("clipboard CF_DIBV5 present but decode failed: {error}");
            None
        }
    }
}

fn read_dib_snapshot() -> Option<ClipboardSnapshot> {
    let dib = read_format_bytes(CF_DIB)?;
    match png_from_dib(&dib) {
        Ok((png_bytes, width, height)) => {
            let snapshot = snapshot_from_png_bytes(png_bytes)?;
            tracing::debug!("read clipboard image from CF_DIB ({width}x{height})");
            Some(snapshot)
        }
        Err(error) => {
            tracing::warn!("clipboard CF_DIB present but decode failed: {error}");
            None
        }
    }
}

/// Writes PNG bytes to the Windows clipboard as CF_DIB, optional PNG, and CF_HDROP staging file.
pub fn write_image(png_bytes: &[u8], _width: u32, _height: u32) -> Result<(), String> {
    let content_hash = *blake3::hash(png_bytes).as_bytes();
    let png_path = staging::image_sync_path(&content_hash);
    staging::write_png_if_absent(&png_path, png_bytes)?;

    let (width, height, rgba) = decode_png_to_rgba(png_bytes)?;
    let dib = rgba_to_cf_dib(&rgba, width, height)?;

    let _clip = Clipboard::new_attempts(10).map_err(|error| format!("open clipboard failed: {error}"))?;

    clipboard_win::raw::empty().map_err(|error| format!("empty clipboard failed: {error}"))?;
    clipboard_win::raw::set_without_clear(CF_DIB, &dib)
        .map_err(|error| format!("write CF_DIB failed: {error}"))?;

    if let Some(png_format) = register_format("PNG") {
        if let Err(error) = clipboard_win::raw::set_without_clear(png_format.get(), png_bytes) {
            tracing::debug!("optional PNG clipboard format skipped: {error}");
        }
    }

    write_hdrop_paths(&[png_path])?;

    tracing::debug!("wrote clipboard image to CF_DIB ({width}x{height}) with HDROP");
    Ok(())
}

pub fn write_file_paths(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("no file paths to write".into());
    }

    let _clip = Clipboard::new_attempts(10).map_err(|error| format!("open clipboard failed: {error}"))?;
    clipboard_win::raw::empty().map_err(|error| format!("empty clipboard failed: {error}"))?;
    write_hdrop_paths(paths)?;
    tracing::debug!("wrote {} clipboard file path(s) to CF_HDROP", paths.len());
    Ok(())
}

fn write_hdrop_paths(paths: &[PathBuf]) -> Result<(), String> {
    let hdrop = build_hdrop_bytes(paths)?;
    clipboard_win::raw::set_without_clear(CF_HDROP, &hdrop)
        .map_err(|error| format!("write CF_HDROP failed: {error}"))
}

const BMP_FILE_HEADER_LEN: usize = 14;
const DROPFILES_HEADER_LEN: usize = 20;

fn rgba_to_cf_dib(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let img = RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| "failed to build RGBA buffer".to_string())?;
    let mut bmp_file = Vec::new();
    img.write_to(&mut Cursor::new(&mut bmp_file), image::ImageFormat::Bmp)
        .map_err(|error| format!("bmp encode failed: {error}"))?;
    if bmp_file.len() <= BMP_FILE_HEADER_LEN {
        return Err("bmp encode produced empty output".into());
    }
    Ok(bmp_file[BMP_FILE_HEADER_LEN..].to_vec())
}

fn build_hdrop_bytes(paths: &[PathBuf]) -> Result<Vec<u8>, String> {
    let mut path_list = Vec::new();
    for path in paths {
        let wide = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>();
        path_list.extend_from_slice(&wide);
    }
    path_list.push(0);

    let mut data = vec![0u8; DROPFILES_HEADER_LEN + path_list.len() * 2];
    let p_files = DROPFILES_HEADER_LEN as u32;
    data[0..4].copy_from_slice(&p_files.to_le_bytes());
    // pt.x/pt.y/fNC stay zero
    data[16..20].copy_from_slice(&1u32.to_le_bytes()); // fWide = TRUE

    for (index, unit) in path_list.iter().enumerate() {
        let offset = DROPFILES_HEADER_LEN + index * 2;
        data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
    }

    Ok(data)
}

fn parse_hdrop_paths(data: &[u8]) -> Option<Vec<PathBuf>> {
    if data.len() < DROPFILES_HEADER_LEN {
        return None;
    }

    let p_files = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize;
    let f_wide = u32::from_le_bytes(data[16..20].try_into().ok()?) != 0;
    if p_files >= data.len() {
        return None;
    }

    let mut paths = Vec::new();
    if f_wide {
        let units = data[p_files..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<u16>>();
        let mut start = 0usize;
        while start < units.len() {
            if units[start] == 0 {
                break;
            }
            let mut end = start;
            while end < units.len() && units[end] != 0 {
                end += 1;
            }
            paths.push(PathBuf::from(String::from_utf16_lossy(&units[start..end])));
            start = end + 1;
        }
    } else {
        let mut start = p_files;
        while start < data.len() {
            if data[start] == 0 {
                break;
            }
            let mut end = start;
            while end < data.len() && data[end] != 0 {
                end += 1;
            }
            paths.push(PathBuf::from(
                String::from_utf8_lossy(&data[start..end]).into_owned(),
            ));
            start = end + 1;
        }
    }

    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdrop_roundtrip_preserves_paths() {
        let paths = vec![
            PathBuf::from(r"C:\Users\test\photo.png"),
            PathBuf::from(r"C:\Users\test\report.pdf"),
        ];
        let encoded = build_hdrop_bytes(&paths).expect("encode hdrop");
        let decoded = parse_hdrop_paths(&encoded).expect("decode hdrop");
        assert_eq!(decoded, paths);
    }
}
