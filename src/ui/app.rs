use crate::core::models::{DbCommand, MediaItem};
use crate::data::db_worker::start_db_worker;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::ui::colors::C_PRIMARY_BG;
use crate::ui::components;
use crate::ui::components::sidebar::sidebar;
use crate::ui::fonts::setup_fonts;
use crate::ui::scan_manager::ScanManager;
use crate::ui::styles::apply_style;
use crate::ui::texture_manager::TextureManager;
use crossbeam_channel::{Receiver, Sender};
use eframe::Frame;
use egui::{Margin, TextureHandle, Ui};
use egui_extras::image::load_image_bytes;
use std::fs;
use std::time::{Duration, Instant};

pub struct MediaApp {
    // core
    db_tx: Sender<DbCommand>,
    pub config: AppConfig,
    pub texture_manager: TextureManager,

    // UI state
    pub search_input: String,
    pub root_path: String,
    pub settings_open: Option<bool>,

    // data
    pub scan_manager: ScanManager,
    pub displayed_items: Vec<MediaItem>,

    pending_query: Option<Receiver<(u64, Vec<MediaItem>)>>,
    current_query_id: u64,
    next_query_id: u64,
    pub last_input_time: Instant,
    debounce_delay: Duration,

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

        let db_tx = start_db_worker();
        let db_tx_for_scan = db_tx.clone();

        let mut app = Self {
            db_tx,
            config,
            texture_manager: TextureManager::new(&cc.egui_ctx),
            search_input: String::new(),
            root_path,
            displayed_items: Vec::new(),
            settings_open: None,
            scan_manager: ScanManager::new(db_tx_for_scan),
            app_icon,
            pending_query: None,
            current_query_id: 0,
            next_query_id: 1,
            last_input_time: Instant::now(),
            debounce_delay: Duration::from_millis(300),
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
        const PAGE_SIZE: usize = 500;

        let (resp_tx, resp_rx) = crossbeam_channel::bounded(1);

        let query_id = self.next_query_id;
        self.next_query_id += 1;

        self.current_query_id = query_id;

        if self.search_input.trim().is_empty() {
            self.db_tx
                .send(DbCommand::Query {
                    id: query_id,
                    limit: PAGE_SIZE,
                    offset: 0,
                    resp: resp_tx,
                })
                .ok();
        } else {
            self.db_tx
                .send(DbCommand::Search {
                    id: query_id,
                    query: self.search_input.clone(),
                    limit: PAGE_SIZE,
                    offset: 0,
                    resp: resp_tx,
                })
                .ok();
        }

        self.pending_query = Some(resp_rx);
    }

    fn handle_search_input(&mut self) {
        let now = Instant::now();

        if now.duration_since(self.last_input_time) >= self.debounce_delay {
            self.send_query();
        }
    }

    pub fn refresh_items(&mut self) {
        self.last_input_time = Instant::now();

        self.send_query();
    }

    fn poll_db(&mut self, ctx: &egui::Context) {
        let mut need_repaint = false;

        if let Some(rx) = &self.pending_query {
            match rx.try_recv() {
                Ok((id, items)) => {
                    if id == self.current_query_id {
                        self.displayed_items = items;
                    }

                    self.pending_query = None;
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // waiting
                }
                Err(_) => {
                    self.pending_query = None;
                }
            }

            if self.pending_query.is_some() {
                need_repaint = true;
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
