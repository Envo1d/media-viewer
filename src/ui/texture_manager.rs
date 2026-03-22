use crate::core::models::ThumbnailState;
use crate::infra::cache::load_or_generate;
use crate::infra::config::AppConfig;
use crossbeam_channel::{unbounded, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle};
use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::thread;

const THUMB_SIZE: u32 = 120;

pub struct TextureManager {
    // TODO: сделать подгрузку только нужных текстур
    states: HashMap<String, ThumbnailState>,

    queue_tx: Sender<String>,
    result_rx: Receiver<(String, RgbaImage)>,

    placeholder: TextureHandle,
}

impl TextureManager {
    pub fn new(ctx: &Context) -> Self {
        let (queue_tx, queue_rx) = unbounded::<String>();
        let (result_tx, result_rx) = unbounded::<(String, RgbaImage)>();

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
            states: HashMap::new(),
            queue_tx,
            result_rx,
            placeholder,
        }
    }

    pub fn get(&mut self, ctx: &Context, path: &str) -> TextureHandle {
        self.process_results(ctx);

        match self.states.get(path) {
            Some(ThumbnailState::Ready(tex)) => tex.clone(),

            Some(ThumbnailState::Loading) => self.placeholder.clone(),

            None => {
                self.states
                    .insert(path.to_string(), ThumbnailState::Loading);

                self.queue_tx.send(path.to_string()).ok();

                self.placeholder.clone()
            }
        }
    }

    fn process_results(&mut self, ctx: &Context) {
        for (path, img) in self.result_rx.try_iter() {
            let size = [img.width() as usize, img.height() as usize];
            let pixels = img.into_raw();

            let texture = ctx.load_texture(
                &path,
                ColorImage::from_rgba_unmultiplied(size, &pixels),
                Default::default(),
            );

            self.states.insert(path, ThumbnailState::Ready(texture));
        }
    }
}
