use crate::core::windows_thumb::get_thumbnail;
use image::codecs::jpeg::JpegEncoder;
use image::RgbaImage;
use std::fs;
use std::hash::Hasher;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::SystemTime;
use twox_hash::XxHash64;

fn preview_cache_path(cache_dir: &PathBuf, file_path: &str) -> PathBuf {
    let mut hasher = XxHash64::with_seed(0);

    hasher.write(file_path.as_bytes());

    let hash = format!("{:016x}.jpg", hasher.finish());

    cache_dir.join(hash)
}

fn save_as_jpg(path: &PathBuf, img: &RgbaImage) {
    if let Ok(file) = fs::File::create(path) {
        let mut writer = BufWriter::new(file);

        let rgb_img = image::DynamicImage::ImageRgba8(img.clone()).into_rgb8();

        let mut encoder = JpegEncoder::new_with_quality(&mut writer, 50);
        if let Err(_) = encoder.encode_image(&rgb_img) {
            drop(writer);
            let _ = fs::remove_file(path);
        }
    }
}

pub fn load_or_generate(cache_dir: &PathBuf, path: &str, thumb_size: u32) -> Option<RgbaImage> {
    let cache_path = preview_cache_path(cache_dir, path);

    if cache_path.exists() {
        if let Ok(img) = image::open(&cache_path) {
            return Some(img.into_rgba8());
        } else {
            let _ = fs::remove_file(&cache_path);
        }
    }

    if let Some(rgba_img) = get_thumbnail(path, thumb_size) {
        if rgba_img.width() > 64 {
            save_as_jpg(&cache_path, &rgba_img);
        }
        return Some(rgba_img);
    }

    None
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

    let mut current_size: u64 = files.iter().map(|f| f.1).sum();

    if current_size <= max_size_bytes {
        return;
    }

    files.sort_by_key(|f| f.2);

    for (path, size, _) in files {
        if current_size <= max_size_bytes {
            break;
        }
        if fs::remove_file(path).is_ok() {
            current_size -= size;
        }
    }
}
