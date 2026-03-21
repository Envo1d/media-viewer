use crate::infra::cache::{get_cache_dir, preview_cache_path};
use egui::{Context, TextureHandle};
use image::{DynamicImage, GenericImage, Rgba, RgbaImage};
use std::collections::HashMap;

pub struct TextureManager {
    cache: HashMap<String, TextureHandle>,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get_or_load(&mut self, ctx: &Context, path: &str) -> TextureHandle {
        if let Some(tex) = self.cache.get(path) {
            return tex.clone();
        }

        let cache_dir = get_cache_dir();
        let cache_path = preview_cache_path(&cache_dir, path);

        let final_img: RgbaImage = if cache_path.exists() {
            image::open(&cache_path)
                .unwrap_or_else(|_| DynamicImage::from(RgbaImage::new(120, 120)))
                .to_rgba8()
        } else {
            let img = image::open(path)
                .unwrap_or_else(|_| DynamicImage::from(RgbaImage::new(120, 120)))
                .to_rgba8();

            let target_size = 120;
            let scaled = image::imageops::thumbnail(&img, target_size, target_size);

            let mut canvas =
                RgbaImage::from_pixel(target_size, target_size, Rgba([200, 200, 200, 255]));
            let x_offset = (target_size - scaled.width()) / 2;
            let y_offset = (target_size - scaled.height()) / 2;
            canvas.copy_from(&scaled, x_offset, y_offset).unwrap();

            let _ = canvas.save(&cache_path);

            canvas
        };

        let pixels: Vec<_> = final_img.pixels().flat_map(|p| p.0).collect();
        let size = [final_img.width() as usize, final_img.height() as usize];
        let texture = ctx.load_texture(
            path,
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            Default::default(),
        );

        self.cache.insert(path.to_string(), texture.clone());
        texture
    }
}
