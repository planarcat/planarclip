use std::path::Path;

use image::GenericImageView;

use crate::clipboard::file::is_image_file_name;
use crate::storage::history_thumbs;

pub const PREVIEW_MAX_BYTES: u64 = 10 * 1024 * 1024;
const PREVIEW_MAX_EDGE: u32 = 480;

pub struct PreviewResult {
    pub kind: &'static str,
    pub thumbnail_ref: String,
}

pub fn generate_and_store_preview(
    path: &Path,
    file_name: &str,
    size_bytes: u64,
    entry_id: &str,
) -> Option<PreviewResult> {
    if !path.is_file() {
        return None;
    }

    let png_bytes;
    let kind;

    if size_bytes <= PREVIEW_MAX_BYTES {
        if let Some(bytes) = content_thumbnail_bytes(path, file_name) {
            png_bytes = bytes;
            kind = "thumbnail";
        } else {
            let icon_bytes = file_icon_bytes(path)?;
            png_bytes = icon_bytes;
            kind = "icon";
        }
    } else {
        let icon_bytes = file_icon_bytes(path)?;
        png_bytes = icon_bytes;
        kind = "icon";
    }

    let thumbnail_ref = history_thumbs::write_png(entry_id, &png_bytes).ok()?;
    Some(PreviewResult { kind, thumbnail_ref })
}

fn content_thumbnail_bytes(path: &Path, file_name: &str) -> Option<Vec<u8>> {
    #[cfg(windows)]
    {
        if let Some(bytes) = crate::platform::windows::thumbnail::shell_content_thumbnail(path, PREVIEW_MAX_EDGE)
        {
            return Some(bytes);
        }
    }

    if is_image_file_name(file_name) {
        return decode_image_thumbnail(path);
    }

    None
}

fn file_icon_bytes(path: &Path) -> Option<Vec<u8>> {
    #[cfg(windows)]
    {
        return crate::platform::windows::thumbnail::shell_file_icon(path, 128);
    }

    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

fn decode_image_thumbnail(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    let (width, height) = image.dimensions();
    let preview = if width <= PREVIEW_MAX_EDGE && height <= PREVIEW_MAX_EDGE {
        image
    } else {
        image.resize(
            PREVIEW_MAX_EDGE,
            PREVIEW_MAX_EDGE,
            image::imageops::FilterType::Triangle,
        )
    };
    let rgba = preview.to_rgba8();
    let mut png_bytes = Vec::new();
    rgba.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .ok()?;
    Some(png_bytes)
}
