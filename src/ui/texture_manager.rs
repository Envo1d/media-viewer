use crate::core::models::TextureTask;
use crate::infra::cache::load_or_generate;
use crate::infra::config::AppConfig;

use crossbeam_channel::{bounded, select_biased, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle};
use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::collections::{HashSet, VecDeque};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread::JoinHandle;
use std::time::Instant;

const THUMB_SIZE: u32 = 120;
const MAX_READY: usize = 600;
const VISIBLE_QUEUE: usize = 64;
const PREFETCH_QUEUE: usize = 128;
const RESULT_QUEUE: usize = 512;
const UPLOAD_BUDGET_US: u128 = 3_500;

pub struct TextureManager {
    ready: HashMap<String, TextureHandle>,
    loading: HashSet<String>,
    failed: HashSet<String>,

    eviction_order: VecDeque<(String, u64)>,
    insert_seq: HashMap<String, u64>,
    seq_counter: u64,

    // Worker pipeline
    visible_tx: Option<Sender<TextureTask>>,
    prefetch_tx: Option<Sender<TextureTask>>,
    result_rx: Receiver<(String, Option<RgbaImage>)>,

    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,

    generation: Arc<AtomicU64>,

    placeholder: TextureHandle,
}

impl TextureManager {
    pub fn new(ctx: &Context) -> Self {
        let (visible_tx, visible_rx) = bounded::<TextureTask>(VISIBLE_QUEUE);
        let (prefetch_tx, prefetch_rx) = bounded::<TextureTask>(PREFETCH_QUEUE);
        let (result_tx, result_rx) = bounded::<(String, Option<RgbaImage>)>(RESULT_QUEUE);

        let cache_dir = AppConfig::get_cache_dir();
        let shutdown = Arc::new(AtomicBool::new(false));
        let generation = Arc::new(AtomicU64::new(0));
        let mut workers = Vec::new();

        let worker_count = num_cpus::get().clamp(2, 4);

        for _ in 0..worker_count {
            let visible_rx = visible_rx.clone();
            let prefetch_rx = prefetch_rx.clone();
            let result_tx = result_tx.clone();
            let shutdown = shutdown.clone();
            let generation = generation.clone();
            let cache_dir = cache_dir.clone();

            let handle = std::thread::Builder::new()
                .name("nexa-thumb-worker".into())
                .spawn(move || {
                    #[cfg(windows)]
                    unsafe {
                        use windows::Win32::System::Com::*;
                        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                    }

                    loop {
                        if shutdown.load(Ordering::Relaxed) {
                            break;
                        }

                        let task = select_biased! {
                            recv(visible_rx) -> r => match r {
                                Ok(t)  => t,
                                Err(_) => break,
                            },
                            recv(prefetch_rx) -> r => match r {
                                Ok(t)  => t,
                                Err(_) => break,
                            },
                        };

                        if task.generation != generation.load(Ordering::Relaxed) {
                            continue;
                        }

                        let img = load_or_generate(&cache_dir, &task.path, THUMB_SIZE);

                        let _ = result_tx.try_send((task.path, img));
                    }

                    #[cfg(windows)]
                    unsafe {
                        use windows::Win32::System::Com::*;
                        CoUninitialize();
                    }
                })
                .expect("Failed to spawn thumbnail worker");

            workers.push(handle);
        }

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
            ready: HashMap::with_capacity(MAX_READY + 64),
            loading: HashSet::new(),
            failed: HashSet::new(),
            eviction_order: VecDeque::with_capacity(MAX_READY + 64),
            insert_seq: HashMap::with_capacity(MAX_READY + 64),
            seq_counter: 0,
            visible_tx: Some(visible_tx),
            prefetch_tx: Some(prefetch_tx),
            result_rx,
            workers,
            shutdown,
            generation,
            placeholder,
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        self.process_results(ctx);
    }

    pub fn get(&mut self, path: &str) -> TextureHandle {
        if let Some(tex) = self.ready.get(path) {
            return tex.clone();
        }

        if self.loading.contains(path) {
            return self.placeholder.clone();
        }

        if self.failed.contains(path) {
            return self.placeholder.clone();
        }

        let task = TextureTask {
            path: path.to_string(),
            priority: 0,
            generation: self.generation.load(Ordering::Relaxed),
            timestamp: Instant::now(),
        };

        if self.visible_tx.as_ref().unwrap().try_send(task).is_ok() {
            self.loading.insert(path.to_string());
        }

        self.placeholder.clone()
    }

    pub fn prefetch(&mut self, path: &str) {
        let visible_pending = self.visible_tx.as_ref().map(|tx| tx.len()).unwrap_or(0);
        if visible_pending > 0 {
            return;
        }

        if self.ready.contains_key(path)
            || self.loading.contains(path)
            || self.failed.contains(path)
        {
            return;
        }

        let task = TextureTask {
            path: path.to_string(),
            priority: 10,
            generation: self.generation.load(Ordering::Relaxed),
            timestamp: Instant::now(),
        };
        if self.prefetch_tx.as_ref().unwrap().try_send(task).is_ok() {
            self.loading.insert(path.to_string());
        }
    }

    pub fn invalidate_prefetch(&mut self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        self.loading.clear();
        self.failed.clear();
    }

    fn process_results(&mut self, ctx: &Context) {
        let deadline = Instant::now() + std::time::Duration::from_micros(UPLOAD_BUDGET_US as u64);
        let mut uploaded = 0usize;

        loop {
            if Instant::now() >= deadline {
                break;
            }

            match self.result_rx.try_recv() {
                Ok((path, Some(img))) => {
                    self.loading.remove(&path);

                    if self.ready.len() >= MAX_READY {
                        self.evict_one();
                    }

                    self.seq_counter += 1;
                    let seq = self.seq_counter;

                    let size = [img.width() as usize, img.height() as usize];
                    let tex = ctx.load_texture(
                        &path,
                        ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                        Default::default(),
                    );
                    self.ready.insert(path.clone(), tex);
                    self.insert_seq.insert(path.clone(), seq);
                    self.eviction_order.push_back((path, seq));

                    uploaded += 1;
                }

                Ok((path, None)) => {
                    self.loading.remove(&path);
                    self.failed.insert(path);
                    uploaded += 1;
                }

                Err(_) => break,
            }
        }

        if uploaded > 0 {
            ctx.request_repaint();
        }
    }

    fn evict_one(&mut self) {
        while let Some((path, seq)) = self.eviction_order.pop_front() {
            if self.insert_seq.get(&path) == Some(&seq) {
                self.ready.remove(&path);
                self.insert_seq.remove(&path);
                return;
            }
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
