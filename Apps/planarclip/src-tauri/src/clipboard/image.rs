use std::io::Cursor;

use arboard::ImageData;

pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
pub const INLINE_IMAGE_BYTES: usize = 512 * 1024;

pub fn encode_rgba_to_png(image: &ImageData<'_>) -> Result<Vec<u8>, String> {
    let width = u32::try_from(image.width).map_err(|_| "image width out of range".to_string())?;
    let height = u32::try_from(image.height).map_err(|_| "image height out of range".to_string())?;
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "image dimensions overflow".to_string())? as usize;
    if image.bytes.len() != expected_len {
        return Err("image byte length mismatch".into());
    }

    let buffer = image::RgbaImage::from_raw(width, height, image.bytes.to_vec())
        .ok_or_else(|| "failed to build RGBA buffer".to_string())?;

    let mut png_bytes = Vec::new();
    buffer
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|error| format!("png encode failed: {error}"))?;

    Ok(png_bytes)
}

pub fn decode_png_to_rgba(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let image = image::load_from_memory(png_bytes).map_err(|error| format!("png decode failed: {error}"))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((width, height, rgba.into_raw()))
}

pub fn format_byte_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
