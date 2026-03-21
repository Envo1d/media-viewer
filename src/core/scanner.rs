use crate::core::models::{MediaItem, MediaType};
use crate::data::db::Database;
use std::fs;
use walkdir::WalkDir;

pub struct MediaScanner<'a> {
    db: &'a mut Database,
}

impl<'a> MediaScanner<'a> {
    pub fn new(db: &'a mut Database) -> Self {
        Self { db }
    }

    pub fn scan_directory(&mut self, root_path: &str) {
        let mut seen_paths = Vec::new();

        for entry in WalkDir::new(root_path)
            .min_depth(3) // scan depth
            .into_iter()
            .filter_map(Result::ok)
        {
            if let Some(item) = self.process_entry(root_path, &entry) {
                seen_paths.push(item.path.clone());
                self.db.upsert(&item);
            }
        }

        self.db.delete_missing(&seen_paths);
    }

    fn process_entry(&self, root_path: &str, entry: &walkdir::DirEntry) -> Option<MediaItem> {
        let path = entry.path();
        if !path.is_file() {
            return None;
        }

        let ext = path.extension()?.to_str()?.to_lowercase();
        let media_type = match ext.as_str() {
            "mp4" | "mkv" | "avi" => MediaType::Video,
            "jpg" | "png" | "jpeg" | "gif" => MediaType::Image,
            _ => return None,
        };

        let metadata = fs::metadata(path).ok()?;
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs() as i64;

        let rel = path.strip_prefix(root_path).ok()?;
        let parts: Vec<_> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect();

        if parts.len() < 3 {
            return None;
        }

        Some(MediaItem {
            path: path.to_string_lossy().to_string(),
            name: path.file_name()?.to_string_lossy().to_string(),
            media_type,
            category: parts[0].to_string(),
            author: parts[1].to_string(),
            modified,
        })
    }
}
