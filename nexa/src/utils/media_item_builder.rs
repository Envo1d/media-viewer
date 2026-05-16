use crate::core::models::{MediaItem, MediaType};
use crate::infra::config::FolderMapping;
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

pub fn extract_characters(stem: &str, separator: &str) -> Vec<String> {
    if separator.is_empty() {
        return Vec::new();
    }

    let s = match stem.rfind('[') {
        Some(pos) => stem[..pos].trim_end(),
        None => stem,
    };

    let s = match s.find(" - ") {
        Some(pos) => s[..pos].trim_end(),
        None => s,
    };

    s.split(separator)
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn build_media_item(
    root: &str,
    path: &Path,
    mapping: &FolderMapping,
    character_separator: &str,
) -> Option<Arc<MediaItem>> {
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

    let folder_count = parts.len().saturating_sub(1);
    if folder_count < mapping.min_folder_depth() {
        return None;
    }

    let copyright = parts
        .get(mapping.copyright_depth)
        .cloned()
        .unwrap_or_default();
    let artist = parts.get(mapping.artist_depth).cloned().unwrap_or_default();

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let characters = extract_characters(&stem, character_separator);

    Some(Arc::new(MediaItem {
        path: path.to_string_lossy().to_string(),
        name: path.file_name()?.to_string_lossy().to_string(),
        media_type,
        copyright,
        artist,
        characters,
        tags: Vec::new(),
        modified,
    }))
}
