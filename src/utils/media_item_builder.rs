use crate::core::models::{MediaItem, MediaType};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

pub fn media_type_from_ext(ext: &str) -> Option<MediaType> {
    match ext {
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => Some(MediaType::Video),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif" => Some(MediaType::Image),
        _ => None,
    }
}

pub fn is_media_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| media_type_from_ext(&e.to_lowercase()).is_some())
}

pub fn build_media_item(root: &str, path: &Path) -> Option<Arc<MediaItem>> {
    if !path.is_file() {
        return None;
    }

    let ext = path.extension()?.to_str()?.to_lowercase();
    let media_type = media_type_from_ext(&ext)?;

    let metadata = fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;

    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if parts.len() < 3 {
        return None;
    }

    Some(Arc::new(MediaItem {
        path: path.to_string_lossy().to_string(),
        name: path.file_name()?.to_string_lossy().to_string(),
        media_type,
        category: parts[0].clone(),
        author: parts[1].clone(),
        modified,
    }))
}
