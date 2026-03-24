use crate::infra::cache::load_or_generate;
use crate::infra::config::AppConfig;
use crossbeam_channel::{bounded, select, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle};
use image::{Rgba, RgbaImage};
use lru::LruCache;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::thread;

const THUMB_SIZE: u32 = 120;
const MAX_TEXTURES: usize = 400;
const QUEUE_LIMIT: usize = 300;

pub struct TextureManager {
    // LRU cache (GPU textures)
    cache: LruCache<String, TextureHandle>,

    // loading state
    loading: HashSet<String>,

    // worker communication
    high_tx: Sender<String>,
    low_tx: Sender<String>,
    result_rx: Receiver<(String, Option<RgbaImage>)>,
    failed: HashSet<String>,

    // other
    placeholder: TextureHandle,
}

impl TextureManager {
    pub fn new(ctx: &Context) -> Self {
        let (high_tx, high_rx) = bounded::<String>(QUEUE_LIMIT);
        let (low_tx, low_rx) = bounded::<String>(QUEUE_LIMIT);

        let (result_tx, result_rx) = bounded::<(String, Option<RgbaImage>)>(QUEUE_LIMIT);

        let cache_dir_base = AppConfig::get_cache_dir();

        // Workers
        let worker_count = std::cmp::min(6, num_cpus::get());

        for _ in 0..worker_count {
            let high_rx = high_rx.clone();
            let low_rx = low_rx.clone();
            let result_tx = result_tx.clone();
            let cache_dir = cache_dir_base.clone();

            thread::spawn(move || {
                unsafe {
                    use windows::Win32::System::Com::*;
                    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
                }

                let process = |path: String| {
                    let img_opt = load_or_generate(&cache_dir, &path, THUMB_SIZE);

                    let _ = result_tx.send((path, img_opt));
                };

                loop {
                    if let Ok(path) = high_rx.try_recv() {
                        process(path);
                        continue;
                    }

                    select! {
                        recv(high_rx) -> msg => {
                            if let Ok(path) = msg { process(path); } else { break; }
                        }
                        recv(low_rx) -> msg => {
                            if let Ok(path) = msg { process(path); } else { break; }
                        }
                    }
                }
            });
        }

        // Placeholder
        let placeholder = {
            let img = RgbaImage::from_pixel(THUMB_SIZE, THUMB_SIZE, Rgba([80, 80, 80, 255]));

            let pixels: Vec<_> = img.pixels().flat_map(|p| p.0).collect();

            ctx.load_texture(
                "placeholder",
                ColorImage::from_rgba_unmultiplied(
                    [THUMB_SIZE as usize, THUMB_SIZE as usize],
                    &pixels,
                ),
                Default::default(),
            )
        };

        Self {
            cache: LruCache::new(NonZeroUsize::new(MAX_TEXTURES).unwrap()),
            loading: HashSet::new(),
            high_tx,
            low_tx,
            result_rx,
            failed: HashSet::new(),
            placeholder,
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        self.process_results(ctx);
    }

    pub fn get(&mut self, _ctx: &Context, path: &str) -> TextureHandle {
        // cache hit
        if let Some(tex) = self.cache.get(path) {
            return tex.clone();
        }

        if self.loading.contains(path) || self.failed.contains(path) {
            return self.placeholder.clone();
        }

        let path_str = path.to_string();

        if self.high_tx.try_send(path_str.clone()).is_ok() {
            self.loading.insert(path_str);
        }

        self.placeholder.clone()
    }

    pub fn prefetch(&mut self, path: &str) {
        if self.cache.get(path).is_some()
            || self.loading.contains(path)
            || self.failed.contains(path)
        {
            return;
        }

        let path_str = path.to_string();

        if self.low_tx.try_send(path_str.clone()).is_ok() {
            self.loading.insert(path_str);
        }
    }

    fn process_results(&mut self, ctx: &Context) {
        let mut processed = 0;
        let max_per_frame = 32;

        for (path, img_opt) in self.result_rx.try_iter() {
            if processed >= max_per_frame {
                break;
            }

            self.loading.remove(&path);

            if let Some(img) = img_opt {
                let size = [img.width() as usize, img.height() as usize];
                let pixels = img.as_raw();

                let texture = ctx.load_texture(
                    &path,
                    ColorImage::from_rgba_unmultiplied(size, &pixels),
                    Default::default(),
                );

                self.failed.remove(&path);
                self.cache.put(path, texture);
            } else {
                self.failed.insert(path);
            }

            processed += 1;
        }

        if processed > 0 {
            ctx.request_repaint();
        }
    }
}
