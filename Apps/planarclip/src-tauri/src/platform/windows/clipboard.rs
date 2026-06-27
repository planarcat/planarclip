use clipboard_win::formats::{CF_DIB, CF_DIBV5, RawData};
use clipboard_win::{is_format_avail, register_format, Clipboard, Setter};

use crate::clipboard::image::{decode_png_to_rgba, png_from_dib, snapshot_from_png_bytes};
use crate::clipboard::types::ClipboardSnapshot;

const BITMAPINFOHEADER_SIZE: usize = 40;

/// Reads image data from the Windows clipboard, including formats that arboard skips (e.g. CF_DIB).
pub fn read_snapshot() -> Option<ClipboardSnapshot> {
    let _clipboard = Clipboard::new_attempts(10).ok()?;

    read_png_snapshot()
        .or_else(read_dibv5_snapshot)
        .or_else(read_dib_snapshot)
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
    let (png_bytes, width, height) = png_from_dib(&dib).ok()?;
    let snapshot = snapshot_from_png_bytes(png_bytes)?;
    tracing::debug!("read clipboard image from CF_DIBV5 ({width}x{height})");
    Some(snapshot)
}

fn read_dib_snapshot() -> Option<ClipboardSnapshot> {
    let dib = read_format_bytes(CF_DIB)?;
    let (png_bytes, width, height) = png_from_dib(&dib).ok()?;
    let snapshot = snapshot_from_png_bytes(png_bytes)?;
    tracing::debug!("read clipboard image from CF_DIB ({width}x{height})");
    Some(snapshot)
}

/// Writes PNG bytes to the Windows clipboard as CF_DIB (and PNG when supported).
pub fn write_image(png_bytes: &[u8], _width: u32, _height: u32) -> Result<(), String> {
    let (width, height, rgba) = decode_png_to_rgba(png_bytes)?;
    let dib = rgba_to_cf_dib(&rgba, width, height)?;

    let _clip = Clipboard::new_attempts(10).map_err(|error| format!("open clipboard failed: {error}"))?;

    RawData(CF_DIB)
        .write_clipboard(&dib)
        .map_err(|error| format!("write CF_DIB failed: {error}"))?;

    if let Some(png_format) = register_format("PNG") {
        if let Err(error) = RawData(png_format.get()).write_clipboard(&png_bytes) {
            tracing::debug!("optional PNG clipboard format skipped: {error}");
        }
    }

    tracing::debug!("wrote clipboard image to CF_DIB ({width}x{height})");
    Ok(())
}

fn rgba_to_cf_dib(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let width_usize = usize::try_from(width).map_err(|_| "image width out of range".to_string())?;
    let height_usize = usize::try_from(height).map_err(|_| "image height out of range".to_string())?;
    let expected_len = width_usize
        .checked_mul(height_usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "image dimensions overflow".to_string())?;
    if rgba.len() != expected_len {
        return Err("image byte length mismatch".into());
    }

    let row_bytes = width_usize
        .checked_mul(4)
        .ok_or_else(|| "image row size overflow".to_string())?;
    let stride = (row_bytes + 3) & !3;
    let image_size = stride
        .checked_mul(height_usize)
        .ok_or_else(|| "image size overflow".to_string())?;

    let mut dib = Vec::with_capacity(BITMAPINFOHEADER_SIZE + image_size);
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(width as i32).to_le_bytes());
    // Negative height stores rows top-down, matching typical screenshot buffers.
    dib.extend_from_slice(&(-(height as i32)).to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&32u16.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&(image_size as u32).to_le_bytes());
    dib.resize(BITMAPINFOHEADER_SIZE, 0);

    for y in 0..height_usize {
        for x in 0..width_usize {
            let index = (y * width_usize + x) * 4;
            dib.push(rgba[index + 2]);
            dib.push(rgba[index + 1]);
            dib.push(rgba[index]);
            dib.push(rgba[index + 3]);
        }
        dib.extend(std::iter::repeat_n(0u8, stride - row_bytes));
    }

    Ok(dib)
}
