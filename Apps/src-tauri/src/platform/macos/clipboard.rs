use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::ClassType;
use objc2_app_kit::{NSPasteboard, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSString, NSURL};

/// Returns the pasteboard change count (similar to Windows clipboard sequence).
pub fn current_sequence() -> Option<u32> {
    let pasteboard = NSPasteboard::generalPasteboard();
    Some(pasteboard.changeCount() as u32)
}

pub fn has_file_format() -> bool {
    let pasteboard = NSPasteboard::generalPasteboard();
    let classes = NSArray::from_slice(&[NSURL::class()]);
    // SAFETY: `classes` contains NSURL.
    unsafe { pasteboard.canReadObjectForClasses_options(&classes, None) }
}

/// Reads file paths from the macOS pasteboard when file URLs are present.
pub fn read_file_paths() -> Option<Vec<PathBuf>> {
    if !has_file_format() {
        return None;
    }

    let pasteboard = NSPasteboard::generalPasteboard();
    let classes = NSArray::from_slice(&[NSURL::class()]);
    let objects = unsafe { pasteboard.readObjectsForClasses_options(&classes, None) }?;

    let mut paths = Vec::new();
    for index in 0..objects.count() {
        let object = objects.objectAtIndex(index);
        let url = object.downcast::<NSURL>().ok()?;
        if !url.isFileURL() {
            continue;
        }
        let path = url.path()?;
        let path_string = path.to_string();
        if !path_string.is_empty() {
            paths.push(PathBuf::from(path_string));
        }
    }

    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}

pub fn write_file_paths(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("no file paths to write".into());
    }

    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();

    let mut writers: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> =
        Vec::with_capacity(paths.len());
    for path in paths {
        let path_string = path.to_string_lossy();
        let ns_path = NSString::from_str(&path_string);
        let url = NSURL::fileURLWithPath_isDirectory(&ns_path, false);
        writers.push(ProtocolObject::from_retained(url));
    }

    let objects = NSArray::from_retained_slice(&writers);
    if pasteboard.writeObjects(&objects) {
        tracing::debug!("wrote {} clipboard file path(s) to NSPasteboard", paths.len());
        Ok(())
    } else {
        Err("write NSPasteboard file URLs failed".into())
    }
}
