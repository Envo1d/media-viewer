use crate::core::models::TextureTask;
use crate::infra::cache::load_or_generate;
use crate::infra::config::AppConfig;

use crossbeam_channel::{bounded, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle};
use image::{Rgba, RgbaImage};

use std::cell::RefCell;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{spawn, JoinHandle};

use lru::LruCache;

const THUMB_SIZE: u32 = 120;
const MAX_TEXTURES: usize = 400;

pub struct TextureManager {
    cache: RefCell<LruCache<String, TextureHandle>>,
    failed: RefCell<HashSet<String>>,
    loading: RefCell<HashSet<String>>,

    visible_tx: Option<Sender<TextureTask>>,
    prefetch_tx: Option<Sender<TextureTask>>,
    result_rx: Receiver<(String, Option<RgbaImage>)>,

    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,

    placeholder: TextureHandle,
}

impl TextureManager {
    pub fn new(ctx: &Context) -> Self {
        let (visible_tx, visible_rx) = bounded::<TextureTask>(512);
        let (prefetch_tx, prefetch_rx) = bounded::<TextureTask>(512);
        let (result_tx, result_rx) = bounded::<(String, Option<RgbaImage>)>(1024);

        let cache_dir = AppConfig::get_cache_dir();
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::new();

        let worker_count = std::cmp::min(6, num_cpus::get());

        for _ in 0..worker_count {
            let visible_rx = visible_rx.clone();
            let prefetch_rx = prefetch_rx.clone();
            let result_tx = result_tx.clone();
            let shutdown = shutdown.clone();
            let cache_dir = cache_dir.clone();

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

                    let task = match visible_rx.try_recv() {
                        Ok(t) => t,
                        Err(_) => match prefetch_rx.recv() {
                            Ok(t) => t,
                            Err(_) => break,
                        },
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

        // Gray placeholder shown while a thumbnail is loading
        let placeholder = {
            let img = RgbaImage::from_pixel(THUMB_SIZE, THUMB_SIZE, Rgba([80, 80, 80, 255]));
            let pixels: Vec<u8> = img.pixels().flat_map(|p| p.0).collect();
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
            cache: RefCell::new(LruCache::new(NonZeroUsize::new(MAX_TEXTURES).unwrap())),
            failed: RefCell::new(HashSet::new()),
            loading: RefCell::new(HashSet::new()),
            visible_tx: Some(visible_tx),
            prefetch_tx: Some(prefetch_tx),
            result_rx,
            workers,
            shutdown,
            placeholder,
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        self.process_results(ctx);
    }

    pub fn get(&self, path: &str) -> TextureHandle {
        if let Some(tex) = self.cache.borrow_mut().get(path) {
            return tex.clone();
        }

        if self.failed.borrow().contains(path) {
            return self.placeholder.clone();
        }

        let mut loading = self.loading.borrow_mut();
        if !loading.contains(path) {
            let task = TextureTask {
                path: path.to_string(),
                priority: 0,
                timestamp: std::time::Instant::now(),
            };
            if self.visible_tx.as_ref().unwrap().send(task).is_ok() {
                loading.insert(path.to_string());
            }
        }

        self.placeholder.clone()
    }

    pub fn prefetch(&self, path: &str) {
        if self.cache.borrow().contains(path) {
            return;
        }
        let mut loading = self.loading.borrow_mut();
        if loading.contains(path) {
            return;
        }
        let task = TextureTask {
            path: path.to_string(),
            priority: 10,
            timestamp: std::time::Instant::now(),
        };
        if self.prefetch_tx.as_ref().unwrap().send(task).is_ok() {
            loading.insert(path.to_string());
        }
    }

    fn process_results(&mut self, ctx: &Context) {
        let max_per_frame = 32;
        let mut processed = 0;

        loop {
            if processed >= max_per_frame {
                break;
            }

            match self.result_rx.try_recv() {
                Ok((path, img_opt)) => {
                    self.loading.borrow_mut().remove(&path);

                    if let Some(img) = img_opt {
                        let size = [img.width() as usize, img.height() as usize];
                        let texture = ctx.load_texture(
                            &path,
                            ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                            Default::default(),
                        );
                        self.failed.borrow_mut().remove(&path);
                        self.cache.borrow_mut().put(path, texture);
                    } else {
                        self.failed.borrow_mut().insert(path);
                    }

                    processed += 1;
                }
                Err(_) => break,
            }
        }

        if processed > 0 {
            ctx.request_repaint();
        }
    }
}

impl Drop for TextureManager {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        drop(self.visible_tx.take());
        drop(self.prefetch_tx.take());
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}
