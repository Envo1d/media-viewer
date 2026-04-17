use crate::core::windows_thumb::get_thumbnail;
use image::RgbaImage;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use twox_hash::XxHash64;
use webp::Encoder;

fn preview_cache_path(cache_dir: &Path, file_path: &str) -> PathBuf {
    let mut hasher = XxHash64::with_seed(0);

    hasher.write(file_path.as_bytes());

    let hash = format!("{:016x}.webp", hasher.finish());

    cache_dir.join(hash)
}

fn save_as_webp_lossy(path: &Path, img: &RgbaImage, quality: f32) {
    let encoder = Encoder::from_rgba(img, img.width(), img.height());

    let quality = quality.clamp(0.0, 100.0);

    let webp_data = encoder.encode(quality);

    let _ = fs::write(path, &*webp_data);
}

pub fn load_or_generate(cache_dir: &Path, path: &str, thumb_size: u32) -> Option<RgbaImage> {
    let cache_path = preview_cache_path(cache_dir, path);

    if let Ok(data) = fs::read(&cache_path) {
        if let Ok(img) = image::load_from_memory_with_format(&data, image::ImageFormat::WebP) {
            return Some(img.into_rgba8());
        } else {
            let _ = fs::remove_file(&cache_path);
        }
    }

    if let Some(rgba_img) = get_thumbnail(path, thumb_size) {
        if rgba_img.width() > 32 {
            save_as_webp_lossy(&cache_path, &rgba_img, 75.0);
        }
        return Some(rgba_img);
    }

    None
}

pub fn prune_cache_async(cache_dir: PathBuf, max_size_mb: u64) {
    std::thread::Builder::new()
        .name("nexa-cache-prune".into())
        .spawn(move || prune_cache(&cache_dir, max_size_mb))
        .ok();
}

pub fn prune_cache(cache_dir: &PathBuf, max_size_mb: u64) {
    let max_size_bytes = max_size_mb * 1024 * 1024;

    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };

    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if meta.is_file() {
                Some((
                    e.path(),
                    meta.len(),
                    meta.modified().unwrap_or(SystemTime::now()),
                ))
            } else {
                None
            }
        })
        .collect();

    let current_size: u64 = files.iter().map(|f| f.1).sum();
    if current_size <= max_size_bytes {
        return;
    }

    files.sort_unstable_by_key(|f| f.2);

    let mut remaining = current_size;
    for (path, size, _) in files {
        if remaining <= max_size_bytes {
            break;
        }
        if fs::remove_file(path).is_ok() {
            remaining -= size;
        }
    }
}
