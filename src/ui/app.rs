use crate::core::models::MediaItem;
use crate::data::db_service::DbService;
use crate::data::db_worker::init_db;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::ui::colors::C_PRIMARY_BG;
use crate::ui::components;
use crate::ui::components::sidebar::sidebar;
use crate::ui::fonts::setup_fonts;
use crate::ui::scan_manager::ScanManager;
use crate::ui::styles::apply_style;
use crate::ui::texture_manager::TextureManager;
use crossbeam_channel::Receiver;
use eframe::Frame;
use egui::{Margin, TextureHandle, Ui};
use egui_extras::image::load_image_bytes;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct MediaApp {
    // core
    pub config: AppConfig,
    pub texture_manager: TextureManager,

    // UI state
    pub search_input: String,
    pub root_path: String,
    pub settings_open: Option<bool>,

    // data
    pub scan_manager: ScanManager,
    pub displayed_items: Vec<Arc<MediaItem>>,

    pending_queries: Vec<Receiver<(u64, Vec<Arc<MediaItem>>)>>,
    current_query_id: u64,
    pub last_input_time: Instant,
    debounce_delay: Duration,
    page: usize,
    has_more: bool,
    is_loading_more: bool,
    last_search_input: String,

    pub app_icon: Option<TextureHandle>,
}

impl MediaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        apply_style(&cc.egui_ctx);

        let config = AppConfig::load();

        let root_path = config
            .library_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "S:\\test".to_string());

        let cache_dir = AppConfig::get_cache_dir();

        let _ = fs::create_dir_all(&cache_dir);

        cache::prune_cache(&cache_dir, 500);

        let app_icon = {
            let icon_bytes = include_bytes!("../../assets/icon.png");

            if let Ok(image) = load_image_bytes(icon_bytes) {
                Some(
                    cc.egui_ctx
                        .load_texture("app_icon", image, Default::default()),
                )
            } else {
                eprintln!("Error: Unable to load assets/icon.png");
                None
            }
        };

        init_db();

        let mut app = Self {
            config,
            texture_manager: TextureManager::new(&cc.egui_ctx),
            search_input: String::new(),
            root_path,
            displayed_items: Vec::new(),
            settings_open: None,
            scan_manager: ScanManager::new(),
            app_icon,
            pending_queries: Vec::new(),
            current_query_id: 0,
            last_input_time: Instant::now(),
            debounce_delay: Duration::from_millis(300),
            page: 0,
            has_more: true,
            is_loading_more: false,
            last_search_input: String::new(),
        };

        app.refresh_items();

        app
    }

    fn handle_scan_events(&mut self, ctx: &egui::Context) {
        let finished = self.scan_manager.update();

        if finished {
            self.refresh_items();
            ctx.request_repaint();
        }
    }

    fn send_query(&mut self) {
        if self.is_loading_more {
            return;
        }

        const PAGE_SIZE: usize = 500;

        let (id, rx) = if self.search_input.trim().is_empty() {
            DbService::query(PAGE_SIZE, 0)
        } else {
            DbService::search(self.search_input.clone(), PAGE_SIZE, 0)
        };

        self.page = 0;
        self.has_more = true;
        self.current_query_id = id;
        self.displayed_items.clear();
        self.pending_queries.clear();
        self.pending_queries.push(rx);
        self.is_loading_more = true;
    }

    fn handle_search_input(&mut self) {
        let now = Instant::now();

        if self.search_input.trim() == self.last_search_input.trim() {
            return;
        }

        // debounce
        if now.duration_since(self.last_input_time) >= self.debounce_delay {
            self.last_search_input = self.search_input.clone();
            self.send_query();
        }
    }

    pub fn refresh_items(&mut self) {
        self.last_input_time = Instant::now();

        self.send_query();
    }

    pub fn load_next_page(&mut self) {
        if !self.has_more || self.is_loading_more {
            return;
        }

        const PAGE_SIZE: usize = 100;

        self.is_loading_more = true;

        let offset = self.page * PAGE_SIZE;

        let (_id, rx) = if self.search_input.trim().is_empty() {
            DbService::query(PAGE_SIZE, offset)
        } else {
            DbService::search(self.search_input.clone(), PAGE_SIZE, offset)
        };

        self.pending_queries.push(rx);
    }

    fn poll_db(&mut self, ctx: &egui::Context) {
        let mut need_repaint = false;

        let mut i = 0;

        while i < self.pending_queries.len() {
            let mut remove = false;

            match self.pending_queries[i].try_recv() {
                Ok((id, items)) => {
                    if id >= self.current_query_id {
                        if self.displayed_items.is_empty() {
                            self.displayed_items = items.clone();
                        } else {
                            self.displayed_items.extend(items.clone());
                        }

                        if items.len() < 100 {
                            self.has_more = false;
                        } else {
                            self.page += 1;
                        }

                        need_repaint = true;
                    }

                    self.is_loading_more = false;

                    remove = true;
                }

                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // wait
                }

                Err(_) => {
                    remove = true;
                }
            }

            if remove {
                self.pending_queries.remove(i);
            } else {
                i += 1;
            }
        }

        if need_repaint {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for MediaApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.clone();

        self.poll_db(&ctx);
        self.handle_search_input();

        self.texture_manager.begin_frame();

        self.texture_manager.update(&ctx);
        self.handle_scan_events(&ctx);

        let window_frame = egui::Frame::NONE
            .fill(C_PRIMARY_BG)
            .stroke(ctx.global_style().visuals.window_stroke());

        egui::CentralPanel::default()
            .frame(window_frame)
            .show_inside(ui, |ui| {
                egui::Panel::top("custom_bar")
                    .frame(egui::Frame::NONE.corner_radius(egui::CornerRadius {
                        nw: 20,
                        ne: 20,
                        sw: 0,
                        se: 0,
                    }))
                    .show_inside(ui, |ui| {
                        components::custom_title_bar(ui, self);
                    });

                egui::Panel::left("sidebar")
                    .exact_size(240.0)
                    .frame(egui::Frame::NONE.inner_margin(Margin::symmetric(10, 10)))
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        sidebar(self, ui);
                    });

                components::settings_modal(self, ui);

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    components::grid_layout(self, ui);

                    self.texture_manager.end_frame();
                });
            });
    }
}
