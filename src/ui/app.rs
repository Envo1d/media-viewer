use crate::core::models::{FieldFilter, LibraryStats, MediaFilter, MediaItem, SortOrder};
use crate::data::db_service::DbService;
use crate::data::db_worker::init_db;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::ui::colors::C_PRIMARY_BG;
use crate::ui::components;
use crate::ui::components::sidebar::sidebar;
use crate::ui::fonts::setup_fonts;
use crate::ui::icon_registry::IconRegistry;
use crate::ui::scan_manager::ScanManager;
use crate::ui::styles::apply_style;
use crate::ui::texture_manager::TextureManager;
use crossbeam_channel::Receiver;
use eframe::Frame;
use egui::{Context, Margin, TextureHandle, Ui};
use egui_extras::image::load_image_bytes;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

const PAGE_SIZE: usize = 100;
const MAX_DISPLAYED_ITEMS: usize = 5000;

pub struct MediaApp {
    // Core
    pub config: AppConfig,
    pub texture_manager: TextureManager,
    pub icons: Option<IconRegistry>,
    pub app_icon: Option<TextureHandle>,

    // UI state
    pub search_input: String,
    pub root_path: String,
    pub settings_open: Option<bool>,

    // View options
    pub filter: MediaFilter,
    pub sort: SortOrder,
    pub card_size: f32,
    pub show_previews: bool,

    // Active quick-filter from sidebar stats (artist / copyright / tag).
    // Cleared whenever the user changes search_input or media type filter.
    pub field_filter: Option<FieldFilter>,

    // Data
    pub scan_manager: ScanManager,
    pub displayed_items: Vec<Arc<MediaItem>>,

    // Sidebar statistics
    pub sidebar_stats: LibraryStats,
    stats_rx: Option<Receiver<LibraryStats>>,

    // Query machinery
    pending_queries: Vec<(u64, u64, Receiver<(u64, Vec<Arc<MediaItem>>)>)>,
    current_query_id: u64,

    // Pagination & search
    pub last_input_time: Instant,
    debounce_delay: Duration,
    last_search_input: String,
    page: usize,
    has_more: bool,
    is_loading_more: bool,

    // Metadata-editor modal state
    pub metadata_modal_item: Option<Arc<MediaItem>>,
    pub metadata_modal_chars: Vec<String>,
    pub metadata_modal_chars_input: String,
    pub metadata_modal_tags: Vec<String>,
    pub metadata_modal_input: String,
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
            .unwrap_or_default();

        let cache_dir = AppConfig::get_cache_dir();
        let _ = fs::create_dir_all(&cache_dir);
        cache::prune_cache_async(cache_dir, 500);

        let app_icon = {
            let icon_bytes = include_bytes!("../../assets/icons/icon.png");
            match load_image_bytes(icon_bytes) {
                Ok(image) => Some(
                    cc.egui_ctx
                        .load_texture("app_icon", image, Default::default()),
                ),
                Err(_) => {
                    eprintln!("Error: Unable to load assets/icons/icon.png");
                    None
                }
            }
        };

        init_db();

        let mut app = Self {
            config: config.clone(),
            texture_manager: TextureManager::new(&cc.egui_ctx),
            search_input: String::new(),
            root_path: root_path.clone(),
            displayed_items: Vec::new(),
            settings_open: None,
            scan_manager: ScanManager::new(),
            filter: MediaFilter::All,
            sort: SortOrder::NameAsc,
            card_size: 200.0,
            app_icon,
            field_filter: None,
            sidebar_stats: LibraryStats::default(),
            stats_rx: None,
            pending_queries: Vec::new(),
            current_query_id: 0,
            last_input_time: Instant::now(),
            debounce_delay: Duration::from_millis(300),
            last_search_input: String::new(),
            page: 0,
            has_more: true,
            is_loading_more: false,
            icons: Some(IconRegistry::new(&cc.egui_ctx)),
            show_previews: true,
            metadata_modal_item: None,
            metadata_modal_chars: Vec::new(),
            metadata_modal_chars_input: String::new(),
            metadata_modal_tags: Vec::new(),
            metadata_modal_input: String::new(),
        };

        app.refresh_items();
        app.request_stats();

        if !root_path.is_empty() {
            let mapping = config.folder_mapping.clone();
            let char_sep = config.character_separator.clone();
            if config.auto_scan {
                app.scan_manager.start(root_path, mapping, char_sep);
            } else {
                app.scan_manager
                    .start_watching(root_path, mapping, char_sep);
            }
        }

        app
    }

    pub fn toggle_field_filter(&mut self, f: FieldFilter) {
        if self.field_filter.as_ref() == Some(&f) {
            self.field_filter = None;
        } else {
            self.field_filter = Some(f);
        }
        self.texture_manager.invalidate_prefetch();
        self.refresh_items();
    }

    pub fn open_metadata_modal(&mut self, item: Arc<MediaItem>) {
        self.metadata_modal_chars = item.characters.clone();
        self.metadata_modal_chars_input = String::new();
        self.metadata_modal_tags = item.tags.clone();
        self.metadata_modal_input = String::new();
        self.metadata_modal_item = Some(item);
    }

    pub fn save_metadata(&mut self) {
        let Some(item) = self.metadata_modal_item.take() else {
            return;
        };
        let new_chars = std::mem::take(&mut self.metadata_modal_chars);
        let new_tags = std::mem::take(&mut self.metadata_modal_tags);
        self.metadata_modal_chars_input.clear();
        self.metadata_modal_input.clear();

        DbService::update_characters(item.path.clone(), new_chars.clone());
        DbService::update_tags(item.path.clone(), new_tags.clone());
        self.apply_metadata_update(&item.path, new_chars, new_tags);

        self.request_stats();
    }

    fn apply_metadata_update(&mut self, path: &str, chars: Vec<String>, tags: Vec<String>) {
        for arc in &mut self.displayed_items {
            if arc.path == path {
                let mut updated = (**arc).clone();
                updated.characters = chars;
                updated.tags = tags;
                *arc = Arc::new(updated);
                return;
            }
        }
    }

    pub fn rescan(&mut self) {
        if self.root_path.is_empty() {
            return;
        }

        let mapping = self.config.folder_mapping.clone();
        let char_sep = self.config.character_separator.clone();
        self.scan_manager
            .start(self.root_path.clone(), mapping, char_sep);
    }

    fn request_stats(&mut self) {
        self.stats_rx = Some(DbService::query_stats());
    }

    fn poll_stats(&mut self, ctx: &Context) {
        let Some(ref rx) = self.stats_rx else { return };
        match rx.try_recv() {
            Ok(stats) => {
                self.sidebar_stats = stats;
                self.stats_rx = None;
                ctx.request_repaint();
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                self.stats_rx = None;
            }
        }
    }

    fn handle_scan_and_watch_events(&mut self, ctx: &Context) {
        let (scan_finished, watch_changed) = self.scan_manager.update();

        if scan_finished || watch_changed {
            self.texture_manager.invalidate_prefetch();
            self.refresh_items();
            self.request_stats();
            ctx.request_repaint();
        }
    }

    fn send_query(&mut self) {
        if self.is_loading_more {
            return;
        }

        let ff = self.field_filter.clone();

        let (id, rx) = if self.search_input.trim().is_empty() {
            DbService::query(PAGE_SIZE, 0, self.filter.clone(), self.sort.clone(), ff)
        } else {
            DbService::search(
                self.search_input.clone(),
                PAGE_SIZE,
                0,
                self.filter.clone(),
                self.sort.clone(),
                ff,
            )
        };

        self.page = 0;
        self.has_more = true;
        self.current_query_id = id;
        self.displayed_items.clear();
        self.pending_queries.clear();
        self.pending_queries.push((id, id, rx));
        self.is_loading_more = true;
    }

    fn handle_search_input(&mut self, ctx: &Context) {
        if self.search_input.trim() == self.last_search_input.trim() {
            return;
        }

        let elapsed = self.last_input_time.elapsed();

        if elapsed >= self.debounce_delay {
            self.last_search_input = self.search_input.clone();
            self.send_query();
        } else {
            ctx.request_repaint_after(self.debounce_delay - elapsed);
        }
    }

    pub fn refresh_items(&mut self) {
        self.is_loading_more = false;
        self.send_query();
    }

    pub fn load_next_page(&mut self) {
        if !self.has_more || self.is_loading_more {
            return;
        }

        if self.displayed_items.len() >= MAX_DISPLAYED_ITEMS {
            return;
        }

        self.is_loading_more = true;

        let offset = self.page * PAGE_SIZE;
        let snapshot = self.current_query_id;
        let ff = self.field_filter.clone();

        let (db_id, rx) = if self.search_input.trim().is_empty() {
            DbService::query(
                PAGE_SIZE,
                offset,
                self.filter.clone(),
                self.sort.clone(),
                ff,
            )
        } else {
            DbService::search(
                self.search_input.clone(),
                PAGE_SIZE,
                offset,
                self.filter.clone(),
                self.sort.clone(),
                ff,
            )
        };

        self.pending_queries.push((snapshot, db_id, rx));
    }

    fn poll_db(&mut self, ctx: &Context) {
        let mut need_repaint = false;
        let current = self.current_query_id;
        let mut i = 0;

        while i < self.pending_queries.len() {
            let (snapshot_id, db_id, ref rx) = self.pending_queries[i];

            if snapshot_id != current {
                self.pending_queries.swap_remove(i);
                self.is_loading_more = false;
                continue;
            }

            let remove = match rx.try_recv() {
                Ok((response_id, items)) => {
                    if response_id == db_id {
                        if items.len() < PAGE_SIZE {
                            self.has_more = false;
                        } else {
                            self.page += 1;
                        }
                        self.displayed_items.extend(items);
                        need_repaint = true;
                    }
                    self.is_loading_more = false;
                    true
                }
                Err(crossbeam_channel::TryRecvError::Empty) => false,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    eprintln!("[app] poll_db: channel disconnected");
                    self.is_loading_more = false;
                    true
                }
            };

            if remove {
                self.pending_queries.swap_remove(i);
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
        let ctx = ui.ctx().clone();

        self.poll_db(&ctx);
        self.poll_stats(&ctx);
        self.handle_search_input(&ctx);
        self.texture_manager.update(&ctx);
        self.handle_scan_and_watch_events(&ctx);

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
                        components::title_bar(ui, self);
                    });

                egui::Panel::left("sidebar")
                    .exact_size(240.0)
                    .frame(egui::Frame::NONE.inner_margin(Margin::symmetric(10, 10)))
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        sidebar(self, ui);
                    });

                components::settings_modal(self, ui);
                components::metadata_modal(self, ui);

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    components::grid_layout(self, ui);
                });
            });
    }
}
