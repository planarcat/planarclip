use std::io::Cursor;

use image::RgbaImage;
use clipboard_win::formats::{CF_DIB, CF_DIBV5};
use clipboard_win::{is_format_avail, register_format, Clipboard};

use crate::clipboard::image::{decode_png_to_rgba, png_from_dib, snapshot_from_png_bytes};
use crate::clipboard::types::ClipboardSnapshot;

/// Returns the current clipboard sequence number without opening the clipboard.
pub fn current_sequence() -> Option<u32> {
    clipboard_win::seq_num().map(|seq| seq.get())
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

/// Writes PNG bytes to the Windows clipboard as CF_DIB (and PNG when supported).
pub fn write_image(png_bytes: &[u8], _width: u32, _height: u32) -> Result<(), String> {
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

    tracing::debug!("wrote clipboard image to CF_DIB ({width}x{height})");
    Ok(())
}

const BMP_FILE_HEADER_LEN: usize = 14;

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
