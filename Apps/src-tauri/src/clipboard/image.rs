use std::io::Cursor;

use arboard::ImageData;

pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
pub const INLINE_IMAGE_BYTES: usize = 512 * 1024;

pub fn snapshot_from_png_bytes(png_bytes: Vec<u8>) -> Option<crate::clipboard::types::ClipboardSnapshot> {
    if png_bytes.is_empty() {
        return None;
    }

    let (width, height, _) = decode_png_to_rgba(&png_bytes).ok()?;
    if png_bytes.len() > MAX_IMAGE_BYTES {
        tracing::info!(
            "clipboard image skipped locally: {} bytes exceeds limit",
            png_bytes.len()
        );
        return None;
    }

    Some(crate::clipboard::types::ClipboardSnapshot::Image {
        png_bytes,
        width,
        height,
    })
}

pub fn png_from_dib(dib: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    use image::codecs::bmp::BmpDecoder;
    use image::DynamicImage;
    use image::ImageDecoder;
    use std::io::Cursor;

    let decoder = BmpDecoder::new_without_file_header(Cursor::new(dib))
        .map_err(|error| format!("dib decode failed: {error}"))?;
    let (width, height) = decoder.dimensions();
    let rgba = DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("dib image build failed: {error}"))?
        .into_rgba8();
    let png_bytes = rgba8_to_png(&rgba)?;
    Ok((png_bytes, width, height))
}

fn rgba8_to_png(rgba: &image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut png_bytes = Vec::new();
    rgba.write_to(
        &mut Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .map_err(|error| format!("png encode failed: {error}"))?;
    Ok(png_bytes)
}

pub fn encode_rgba_to_png(image: &ImageData<'_>) -> Result<Vec<u8>, String> {
    let width = u32::try_from(image.width).map_err(|_| "image width out of range".to_string())?;
    let height = u32::try_from(image.height).map_err(|_| "image height out of range".to_string())?;
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "image dimensions overflow".to_string())? as usize;
    if image.bytes.len() != expected_len {
        if let Some(tight) = tight_rgba_bytes(image) {
            let buffer = image::RgbaImage::from_raw(width, height, tight)
                .ok_or_else(|| "failed to build RGBA buffer".to_string())?;
            return rgba8_to_png(&buffer);
        }
        return Err(format!(
            "image byte length mismatch: expected {expected_len}, got {}",
            image.bytes.len()
        ));
    }

    let buffer = image::RgbaImage::from_raw(width, height, image.bytes.to_vec())
        .ok_or_else(|| "failed to build RGBA buffer".to_string())?;

    rgba8_to_png(&buffer)
}

fn tight_rgba_bytes(image: &ImageData<'_>) -> Option<Vec<u8>> {
    let width = image.width;
    let height = image.height;
    if width == 0 || height == 0 {
        return None;
    }

    let row_bytes = width.checked_mul(4)?;
    let stride = image.bytes.len() / height;
    if stride < row_bytes || stride * height != image.bytes.len() {
        return None;
    }

    let mut tight = Vec::with_capacity(row_bytes * height);
    for row in 0..height {
        let start = row * stride;
        tight.extend_from_slice(&image.bytes[start..start + row_bytes]);
    }
    Some(tight)
}

pub fn decode_png_to_rgba(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let image = image::load_from_memory(png_bytes).map_err(|error| format!("png decode failed: {error}"))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((width, height, rgba.into_raw()))
}

pub fn png_data_url(png_bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    format!("data:image/png;base64,{}", BASE64.encode(png_bytes))
}

pub fn png_bytes_from_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    let prefix = "data:image/png;base64,";
    let encoded = data_url
        .strip_prefix(prefix)
        .ok_or_else(|| "无法读取该图片的历史数据。".to_string())?;
    BASE64
        .decode(encoded)
        .map_err(|_| "无法读取该图片的历史数据。".to_string())
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
