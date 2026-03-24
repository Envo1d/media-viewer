use crate::core::models::TextureTask;
use crate::infra::cache::load_or_generate;
use crate::infra::config::AppConfig;
use crossbeam_channel::{bounded, Receiver};
use egui::{ColorImage, Context, TextureHandle};
use image::{Rgba, RgbaImage};
use lru::LruCache;
use std::collections::{BinaryHeap, HashSet};
use std::num::NonZeroUsize;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Instant;

const THUMB_SIZE: u32 = 120;
const MAX_TEXTURES: usize = 400;
const QUEUE_LIMIT: usize = 300;

pub struct TextureManager {
    // LRU cache
    cache: LruCache<String, TextureHandle>,

    // state
    loading: HashSet<String>,
    failed: HashSet<String>,

    // scheduler
    task_queue: Arc<(Mutex<BinaryHeap<TextureTask>>, Condvar)>,
    in_queue: Arc<Mutex<HashSet<String>>>,

    // visible
    visible_set: HashSet<String>,

    // results
    result_rx: Receiver<(String, Option<RgbaImage>)>,

    // placeholder
    placeholder: TextureHandle,
}

impl TextureManager {
    pub fn new(ctx: &Context) -> Self {
        let (result_tx, result_rx) = bounded::<(String, Option<RgbaImage>)>(QUEUE_LIMIT);

        let task_queue = Arc::new((Mutex::new(BinaryHeap::<TextureTask>::new()), Condvar::new()));
        let in_queue = Arc::new(Mutex::new(HashSet::new()));

        let cache_dir = AppConfig::get_cache_dir();

        // Workers
        let worker_count = std::cmp::min(6, num_cpus::get());

        for _ in 0..worker_count {
            let queue_pair = task_queue.clone();
            let in_queue = in_queue.clone();
            let result_tx = result_tx.clone();
            let cache_dir = cache_dir.clone();

            thread::spawn(move || {
                #[cfg(windows)]
                unsafe {
                    use windows::Win32::System::Com::*;
                    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
                }

                loop {
                    let task = {
                        let (lock, cvar) = &*queue_pair;
                        let mut q = lock.lock().unwrap();

                        while q.is_empty() {
                            q = cvar.wait(q).unwrap();
                        }
                        q.pop().unwrap()
                    };

                    let path = task.path.clone();

                    let img = load_or_generate(&cache_dir, &path, THUMB_SIZE);

                    in_queue.lock().unwrap().remove(&path);

                    let _ = result_tx.send((path.clone(), img));
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
            failed: HashSet::new(),
            task_queue,
            in_queue,
            visible_set: HashSet::new(),
            result_rx,
            placeholder,
        }
    }

    pub fn begin_frame(&mut self) {
        self.visible_set.clear();
    }

    pub fn end_frame(&mut self) {
        let (lock, _) = &*self.task_queue;
        let mut queue = lock.lock().unwrap();

        if queue.is_empty() {
            return;
        }

        let mut in_q = self.in_queue.lock().unwrap();
        let visible = &self.visible_set;

        let mut tasks: Vec<TextureTask> = std::mem::take(&mut *queue).into_vec();

        tasks.retain(|task| {
            let keep = task.priority == 0 || visible.contains(&task.path);

            if !keep {
                in_q.remove(&task.path);
                self.loading.remove(&task.path);
            }
            keep
        });

        *queue = BinaryHeap::from(tasks);
    }

    pub fn update(&mut self, ctx: &Context) {
        self.process_results(ctx);
    }

    pub fn get(&mut self, _ctx: &Context, path: &str) -> TextureHandle {
        // cache hit
        if let Some(tex) = self.cache.get(path) {
            return tex.clone();
        }

        // mark visible
        self.visible_set.insert(path.to_string());

        if self.failed.contains(path) {
            return self.placeholder.clone();
        }

        let mut in_q = self.in_queue.lock().unwrap();

        if !in_q.contains(path) {
            let (lock, cvar) = &*self.task_queue;
            let mut q = lock.lock().unwrap();

            q.push(TextureTask {
                path: path.to_string(),
                priority: 0,
                timestamp: Instant::now(),
            });

            in_q.insert(path.to_string());
            self.loading.insert(path.to_string());

            cvar.notify_one();
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

        let mut in_q = self.in_queue.lock().unwrap();

        if !in_q.contains(path) {
            let (lock, cvar) = &*self.task_queue;
            let mut q = lock.lock().unwrap();

            q.push(TextureTask {
                path: path.to_string(),
                priority: 0,
                timestamp: Instant::now(),
            });

            in_q.insert(path.to_string());
            self.loading.insert(path.to_string());

            // Будим одного свободного воркера
            cvar.notify_one();
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

                let texture = ctx.load_texture(
                    &path,
                    ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
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
