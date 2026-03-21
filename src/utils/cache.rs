use directories::ProjectDirs;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

pub fn get_cache_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "envoid", "media_viewer") {
        let cache_dir = proj_dirs.config_dir().join("cache");
        fs::create_dir_all(&cache_dir).ok();
        cache_dir
    } else {
        let fallback = PathBuf::from("./cache");
        fs::create_dir_all(&fallback).ok();
        fallback
    }
}

pub fn preview_cache_path(cache_dir: &PathBuf, file_path: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(file_path.as_bytes());
    let hash = format!("{:x}.png", hasher.finalize());
    cache_dir.join(hash)
}
