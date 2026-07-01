use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use image::RgbaImage;
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetDIBits, GetObjectW,
    ReleaseDC, SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    HGDIOBJ, RGBQUAD,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    IShellItem, IShellItemImageFactory, SHCreateItemFromParsingName, SHGetFileInfoW, SHFILEINFOW,
    SHGFI_ICON, SHGFI_LARGEICON, SIIGBF,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON};

const SIIGBF_BIGGERSIZEOK: SIIGBF = SIIGBF(0x1);
const SIIGBF_THUMBNAILONLY: SIIGBF = SIIGBF(0x8);

pub fn shell_content_thumbnail(path: &Path, max_edge: u32) -> Option<Vec<u8>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let result = try_shell_content_thumbnail(path, max_edge);
        CoUninitialize();
        result
    }
}

pub fn shell_file_icon(path: &Path, size: u32) -> Option<Vec<u8>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let result = try_shell_file_icon(path, size);
        CoUninitialize();
        result
    }
}

unsafe fn try_shell_content_thumbnail(path: &Path, max_edge: u32) -> Option<Vec<u8>> {
    let wide = wide_null(path.as_os_str());
    let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).ok()?;
    let factory: IShellItemImageFactory = item.cast().ok()?;
    let size = SIZE {
        cx: max_edge as i32,
        cy: max_edge as i32,
    };
    let hbitmap = factory
        .GetImage(size, SIIGBF_BIGGERSIZEOK | SIIGBF_THUMBNAILONLY)
        .ok()?;
    let png = hbitmap_to_png(hbitmap);
    let _ = DeleteObject(HGDIOBJ(hbitmap.0));
    png
}

unsafe fn try_shell_file_icon(path: &Path, size: u32) -> Option<Vec<u8>> {
    let wide = wide_null(path.as_os_str());
    let mut info = SHFILEINFOW::default();
    let result = SHGetFileInfoW(
        PCWSTR(wide.as_ptr()),
        Default::default(),
        Some(&mut info as *mut _),
        std::mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_ICON | SHGFI_LARGEICON,
    );
    if result == 0 || info.hIcon.0.is_null() {
        return None;
    }

    let png = icon_to_png(info.hIcon, size);
    let _ = DestroyIcon(info.hIcon);
    png
}

unsafe fn icon_to_png(icon: HICON, size: u32) -> Option<Vec<u8>> {
    let dc = GetDC(None);
    if dc.0.is_null() {
        return None;
    }
    let mem_dc = CreateCompatibleDC(Some(dc));
    if mem_dc.0.is_null() {
        ReleaseDC(None, dc);
        return None;
    }

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            biHeight: -(size as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };

    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let hbitmap = CreateDIBSection(
        Some(mem_dc),
        &bmi,
        DIB_RGB_COLORS,
        &mut bits,
        None,
        0,
    )
    .ok()?;

    let old = SelectObject(mem_dc, HGDIOBJ(hbitmap.0));
    let _ = DrawIconEx(
        mem_dc,
        0,
        0,
        icon,
        size as i32,
        size as i32,
        0,
        None,
        DI_NORMAL,
    );

    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let rows = GetDIBits(
        mem_dc,
        hbitmap,
        0,
        size,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    SelectObject(mem_dc, old);
    let _ = DeleteObject(HGDIOBJ(hbitmap.0));
    let _ = DeleteDC(mem_dc);
    ReleaseDC(None, dc);

    if rows == 0 {
        return None;
    }

    bgra_to_rgba(&mut pixels);
    rgba_to_png(size, size, &pixels)
}

unsafe fn hbitmap_to_png(hbitmap: HBITMAP) -> Option<Vec<u8>> {
    let dc = GetDC(None);
    if dc.0.is_null() {
        return None;
    }

    let mut bitmap = BITMAP::default();
    if GetObjectW(
        HGDIOBJ(hbitmap.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bitmap as *mut _ as *mut _),
    ) == 0
    {
        ReleaseDC(None, dc);
        return None;
    }

    let width = bitmap.bmWidth.unsigned_abs();
    let height = bitmap.bmHeight.unsigned_abs();
    if width == 0 || height == 0 {
        ReleaseDC(None, dc);
        return None;
    }

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let rows = GetDIBits(
        dc,
        hbitmap,
        0,
        height,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    ReleaseDC(None, dc);

    if rows == 0 {
        return None;
    }

    bgra_to_rgba(&mut pixels);
    rgba_to_png(width, height, &pixels)
}

fn bgra_to_rgba(pixels: &mut [u8]) {
    for chunk in pixels.chunks_mut(4) {
        chunk.swap(0, 2);
    }
}

fn rgba_to_png(width: u32, height: u32, pixels: &[u8]) -> Option<Vec<u8>> {
    let image = RgbaImage::from_raw(width, height, pixels.to_vec())?;
    let mut png_bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .ok()?;
    Some(png_bytes)
}

fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
