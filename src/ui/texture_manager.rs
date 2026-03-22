use crate::infra::cache::load_or_generate;
use crate::infra::config::AppConfig;
use crossbeam_channel::{bounded, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle};
use image::{Rgba, RgbaImage};
use lru::LruCache;
use std::collections::{HashSet, VecDeque};
use std::num::NonZeroUsize;
use std::thread;

const THUMB_SIZE: u32 = 120;
const MAX_TEXTURES: usize = 150;
const QUEUE_LIMIT: usize = 300;
const MAX_RETRY: usize = 512;

pub struct TextureManager {
    // LRU cache (GPU textures)
    cache: LruCache<String, TextureHandle>,

    // loading state
    loading: HashSet<String>,

    // worker communication
    queue_tx: Sender<String>,
    result_rx: Receiver<(String, RgbaImage)>,

    // retry
    retry_queue: VecDeque<String>,
    retry_set: HashSet<String>,

    // other
    placeholder: TextureHandle,
}

impl TextureManager {
    pub fn new(ctx: &Context) -> Self {
        let (queue_tx, queue_rx) = bounded::<String>(QUEUE_LIMIT);
        let (result_tx, result_rx) = bounded::<(String, RgbaImage)>(QUEUE_LIMIT);

        let cache_dir_base = AppConfig::get_cache_dir();

        // Workers
        for _ in 0..num_cpus::get() {
            let queue_rx = queue_rx.clone();
            let result_tx = result_tx.clone();
            let ctx = ctx.clone();
            let cache_dir = cache_dir_base.clone();

            thread::spawn(move || {
                unsafe {
                    use windows::Win32::System::Com::*;
                    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
                }

                for path in queue_rx {
                    let img = load_or_generate(&cache_dir, &path, THUMB_SIZE);

                    if result_tx.send((path, img)).ok().is_some() {
                        ctx.request_repaint();
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
            queue_tx,
            result_rx,
            retry_queue: VecDeque::new(),
            retry_set: HashSet::new(),
            placeholder,
        }
    }

    pub fn get(&mut self, ctx: &Context, path: &str) -> TextureHandle {
        self.process_results(ctx);

        // cache hit
        if let Some(tex) = self.cache.get(path) {
            return tex.clone();
        }

        // loading → placeholder
        if self.loading.contains(path) {
            return self.placeholder.clone();
        }

        let path_str = path.to_string();

        // 3. try enqueue
        if self.queue_tx.try_send(path_str.clone()).is_ok() {
            self.loading.insert(path_str);
        } else {
            if self.retry_queue.len() < MAX_RETRY && !self.retry_set.contains(&path_str) {
                self.retry_queue.push_back(path_str.clone());
                self.retry_set.insert(path_str);
            }
        }

        self.placeholder.clone()
    }

    fn process_results(&mut self, ctx: &Context) {
        for (path, img) in self.result_rx.try_iter() {
            self.loading.remove(&path);

            let size = [img.width() as usize, img.height() as usize];
            let pixels = img.into_raw();

            let texture = ctx.load_texture(
                &path,
                ColorImage::from_rgba_unmultiplied(size, &pixels),
                Default::default(),
            );

            self.cache.put(path, texture);
        }

        let mut attempts = 0;
        let max_attempts_per_frame = 32;

        while attempts < max_attempts_per_frame {
            let path = match self.retry_queue.pop_front() {
                Some(p) => p,
                None => break,
            };
            
            self.retry_set.remove(&path);
            
            if self.cache.contains(&path) {
                continue;
            }
            
            if self.loading.contains(&path) {
                continue;
            }
            
            if self.queue_tx.try_send(path.clone()).is_ok() {
                self.loading.insert(path);
            } else {
                if self.retry_queue.len() < MAX_RETRY {
                    self.retry_queue.push_back(path.clone());
                    self.retry_set.insert(path);
                }
                break;
            }

            attempts += 1;
        }
    }
}
