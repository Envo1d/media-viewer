use crate::core::windows_thumb::get_thumbnail;
use image::{Rgba, RgbaImage};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn preview_cache_path(cache_dir: &PathBuf, file_path: &str) -> PathBuf {
    let mut hasher = Sha256::new();

    hasher.update(file_path.as_bytes());

    let hash = format!("{:x}.png", hasher.finalize());

    cache_dir.join(hash)
}

fn fallback_image(size: u32) -> RgbaImage {
    RgbaImage::from_pixel(size, size, Rgba([50, 50, 50, 255]))
}

pub fn load_or_generate(cache_dir: &PathBuf, path: &str, thumb_size: u32) -> RgbaImage {
    let cache_path = preview_cache_path(cache_dir, path);

    if cache_path.exists() {
        if let Ok(img) = image::open(&cache_path) {
            return img.into_rgba8();
        }
    }

    if let Some(img) = get_thumbnail(path, thumb_size) {
        let _ = img.save(&cache_path);
        return img;
    }

    fallback_image(thumb_size)
}
