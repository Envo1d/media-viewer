use crate::core::models::TextureTask;
use crate::infra::cache::load_or_generate;
use crate::infra::config::AppConfig;

use crossbeam_channel::{bounded, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle};
use image::{Rgba, RgbaImage};

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::{
    atomic::{AtomicBool, Ordering}, Arc,
    Mutex,
};
use std::thread::{spawn, JoinHandle};
use std::time::Instant;

use lru::LruCache;

const THUMB_SIZE: u32 = 120;
const MAX_TEXTURES: usize = 400;

pub struct TextureManager {
    // cache
    cache: Mutex<LruCache<String, TextureHandle>>,

    // state
    loading: Mutex<HashSet<String>>,
    failed: Mutex<HashSet<String>>,

    // visible set (from UI each frame)
    visible_set: Mutex<HashSet<String>>,

    // task queue
    task_tx: Option<Sender<TextureTask>>,
    result_rx: Receiver<(String, Option<RgbaImage>)>,

    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,

    placeholder: TextureHandle,
}

impl TextureManager {
    pub fn new(ctx: &Context) -> Self {
        let (task_tx, task_rx) = bounded::<TextureTask>(1024);
        let (result_tx, result_rx) = bounded::<(String, Option<RgbaImage>)>(1024);

        let cache_dir = AppConfig::get_cache_dir();

        let shutdown = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::new();

        let worker_count = std::cmp::min(6, num_cpus::get());

        for _ in 0..worker_count {
            let task_rx = task_rx.clone();
            let result_tx = result_tx.clone();
            let cache_dir = cache_dir.clone();
            let shutdown = shutdown.clone();

            let handle = spawn(move || {
                #[cfg(windows)]
                unsafe {
                    use windows::Win32::System::Com::*;
                    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                }

                loop {
                    if shutdown.load(Ordering::Relaxed) {
                        break;
                    }

                    let task = match task_rx.recv() {
                        Ok(t) => t,
                        Err(_) => break,
                    };

                    let img = load_or_generate(&cache_dir, &task.path, THUMB_SIZE);

                    let _ = result_tx.send((task.path, img));
                }

                #[cfg(windows)]
                unsafe {
                    use windows::Win32::System::Com::*;
                    CoUninitialize();
                }
            });

            workers.push(handle);
        }

        // placeholder
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
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(MAX_TEXTURES).unwrap())),
            loading: Mutex::new(HashSet::new()),
            failed: Mutex::new(HashSet::new()),
            visible_set: Mutex::new(HashSet::new()),
            task_tx: Some(task_tx),
            result_rx,
            workers,
            shutdown,
            placeholder,
        }
    }

    pub fn begin_frame(&mut self) {
        self.visible_set.lock().unwrap().clear();
    }

    pub fn update(&mut self, ctx: &Context) {
        self.process_results(ctx);
    }

    pub fn get(&self, path: &str) -> TextureHandle {
        // cache hit
        if let Some(tex) = self.cache.lock().unwrap().get(path) {
            return tex.clone();
        }

        // mark visible
        self.visible_set.lock().unwrap().insert(path.to_string());

        if self.failed.lock().unwrap().contains(path) {
            return self.placeholder.clone();
        }

        let mut loading = self.loading.lock().unwrap();

        if !loading.contains(path) {
            let task = TextureTask {
                path: path.to_string(),
                priority: 0,
                timestamp: Instant::now(),
            };

            let _ = self.task_tx.as_ref().unwrap().send(task);
            loading.insert(path.to_string());
        }

        self.placeholder.clone()
    }

    pub fn prefetch(&self, path: &str) {
        if self.cache.lock().unwrap().get(path).is_some() {
            return;
        }

        let mut loading = self.loading.lock().unwrap();

        if loading.contains(path) {
            return;
        }

        let task = TextureTask {
            path: path.to_string(),
            priority: 10,
            timestamp: Instant::now(),
        };

        if self.task_tx.as_ref().unwrap().send(task).is_ok() {
            loading.insert(path.to_string());
        }
    }

    fn process_results(&mut self, ctx: &Context) {
        let mut processed = 0;
        let max_per_frame = 32;

        while let Ok((path, img_opt)) = self.result_rx.try_recv() {
            if processed >= max_per_frame {
                break;
            }

            self.loading.lock().unwrap().remove(&path);

            if let Some(img) = img_opt {
                let size = [img.width() as usize, img.height() as usize];

                let texture = ctx.load_texture(
                    &path,
                    ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                    Default::default(),
                );

                self.failed.lock().unwrap().remove(&path);
                self.cache.lock().unwrap().put(path.clone(), texture);
            } else {
                self.failed.lock().unwrap().insert(path);
            }

            processed += 1;
        }

        if processed > 0 {
            ctx.request_repaint();
        }
    }
}

impl Drop for TextureManager {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        drop(self.task_tx.take());

        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}
