use crate::core::models::{MediaItem, ScanEvent};
use crate::core::scanner::MediaScanner;
use crate::data::db::Database;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::ui::components;
use crate::ui::texture_manager::TextureManager;
use crossbeam_channel::Receiver;
use std::collections::HashSet;
use std::fs;

const MAX_LIVE_ITEMS: usize = 5000;
const MAX_DISPLAYED: usize = 10000;

pub struct MediaApp {
    // core
    db: Database,
    pub config: AppConfig,
    pub texture_manager: TextureManager,

    // UI state
    search_input: String,
    pub root_path: String,
    pub settings_open: Option<bool>,

    // scanning
    pub is_scanning: bool,
    scan_rx: Option<Receiver<ScanEvent>>,

    // data
    live_items: Vec<MediaItem>,
    pub displayed_items: Vec<MediaItem>,
    seen_paths: HashSet<String>,
}

impl MediaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();

        let root_path = config
            .library_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "S:\\test".to_string());

        let cache_dir = AppConfig::get_cache_dir();

        let _ = fs::create_dir_all(&cache_dir);

        cache::prune_cache(&cache_dir, 500);

        let mut app = Self {
            db: Database::new(),
            config,
            texture_manager: TextureManager::new(&cc.egui_ctx),

            search_input: String::new(),
            root_path,

            scan_rx: None,
            is_scanning: false,

            live_items: Vec::new(),
            displayed_items: Vec::new(),
            seen_paths: HashSet::new(),

            settings_open: None,
        };

        app.refresh_items();

        app
    }

    pub fn start_scan(&mut self) {
        let (tx, rx) = crossbeam_channel::unbounded();

        self.scan_rx = Some(rx);
        self.is_scanning = true;

        self.live_items.clear();
        self.displayed_items.clear();
        self.seen_paths.clear();

        MediaScanner::start(self.root_path.clone(), tx);
    }

    fn handle_scan_events(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.scan_rx {
            let mut added = 0;
            let mut finished = false;

            for event in rx.try_iter() {
                match event {
                    ScanEvent::Item(item) => {
                        if self.seen_paths.insert(item.path.clone()) {
                            self.live_items.push(item.clone());
                            self.displayed_items.push(item);

                            added += 1;

                            if self.displayed_items.len() > MAX_DISPLAYED {
                                self.displayed_items.drain(0..1000);
                            }

                            if self.live_items.len() > MAX_LIVE_ITEMS {
                                self.live_items.remove(0);
                            }
                        }
                    }

                    ScanEvent::Finished => {
                        finished = true;
                    }
                }
            }

            if finished {
                self.is_scanning = false;
                self.refresh_items();
            }

            if added > 0 || finished {
                ctx.request_repaint();
            }
        }
    }

    fn refresh_items(&mut self) {
        const PAGE_SIZE: usize = 500;
        self.displayed_items = if self.search_input.trim().is_empty() {
            self.db.query(PAGE_SIZE, 0)
        } else {
            self.db.search(&self.search_input, PAGE_SIZE, 0)
        };
    }
}

impl eframe::App for MediaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.texture_manager.begin_frame();

        self.texture_manager.update(ctx);
        self.handle_scan_events(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            if ui.button("⚙").clicked() {
                self.settings_open = Some(true);
            }
            ui.text_edit_singleline(&mut self.search_input);
        });

        components::settings_modal(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            components::grid_layout(self, ui);

            self.texture_manager.end_frame();
        });
    }
}
