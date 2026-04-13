use crate::core::models::{
    AutocompleteData, FieldFilter, LibraryStats, MediaFilter, MediaItem, MediaModalMode, MediaType,
    ModalAction, ResolvedName, SortOrder, StagingItem, ViewMode,
};
use crate::data::db_service::DbService;
use crate::data::db_worker::init_db;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::ui::colors::C_PRIMARY_BG;
use crate::ui::components;
use crate::ui::components::media_modal::{media_modal, MediaModalState};
use crate::ui::components::sidebar::sidebar;
use crate::ui::components::staging_sidebar::staging_sidebar;
use crate::ui::components::staging_view::staging_view;
use crate::ui::fonts::setup_fonts;
use crate::ui::icon_registry::IconRegistry;
use crate::ui::scan_manager::ScanManager;
use crate::ui::styles::apply_style;
use crate::ui::texture_manager::TextureManager;
use crate::utils::file_helpers::{build_filename_stem, move_file, resolve_conflict};
use crossbeam_channel::Receiver;
use eframe::Frame;
use egui::{Context, Margin, TextureHandle, Ui};
use egui_extras::image::load_image_bytes;
use std::fs;
use std::path::Path;
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

    // View state
    pub view_mode: ViewMode,
    pub search_input: String,
    pub root_path: String,
    pub settings_open: Option<bool>,

    // View options
    pub filter: MediaFilter,
    pub sort: SortOrder,
    pub card_size: f32,
    pub show_previews: bool,
    pub field_filter: Option<FieldFilter>,

    // Data – main library
    pub scan_manager: ScanManager,
    pub displayed_items: Vec<Arc<MediaItem>>,

    // Sidebar statistics
    pub sidebar_stats: LibraryStats,
    stats_rx: Option<Receiver<LibraryStats>>,

    // Autocomplete data for the distribute modal
    pub autocomplete: AutocompleteData,
    autocomplete_rx: Option<Receiver<AutocompleteData>>,

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

    // Settings input state
    pub character_separator_input: String,
    pub video_subfolder_input: String,

    // Unified modal
    pub modal_state: MediaModalState,

    // Staging
    pub staging_items: Vec<Arc<StagingItem>>,
    staging_rx: Option<Receiver<Vec<Arc<StagingItem>>>>,
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
            let bytes = include_bytes!("../../assets/icons/icon.png");
            match load_image_bytes(bytes) {
                Ok(img) => Some(
                    cc.egui_ctx
                        .load_texture("app_icon", img, Default::default()),
                ),
                Err(_) => {
                    eprintln!("Error: Unable to load assets/icons/icon.png");
                    None
                }
            }
        };

        init_db();

        let staging_path_str = config
            .staging_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let excluded_dirs: Vec<String> = config
            .staging_path
            .as_ref()
            .map(|p| vec![p.to_string_lossy().to_string()])
            .unwrap_or_default();

        let character_separator = config.character_separator.clone();
        let video_subfolder = config.video_subfolder.clone();

        let mut app = Self {
            config: config.clone(),
            texture_manager: TextureManager::new(&cc.egui_ctx),
            search_input: String::new(),
            root_path: root_path.clone(),
            displayed_items: Vec::new(),
            settings_open: None,
            view_mode: ViewMode::Library,
            scan_manager: ScanManager::new(),
            filter: MediaFilter::All,
            sort: SortOrder::NameAsc,
            card_size: 200.0,
            app_icon,
            field_filter: None,
            sidebar_stats: LibraryStats::default(),
            stats_rx: None,
            autocomplete: AutocompleteData::default(),
            autocomplete_rx: None,
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
            staging_items: Vec::new(),
            staging_rx: None,
            modal_state: MediaModalState::default(),
            character_separator_input: character_separator,
            video_subfolder_input: video_subfolder,
        };

        app.refresh_items();
        app.request_stats();
        app.request_autocomplete();
        app.refresh_staging_items();

        if !root_path.is_empty() {
            let mapping = config.folder_mapping.clone();
            let char_sep = config.character_separator.clone();
            if config.auto_scan {
                app.scan_manager
                    .start(root_path, mapping, char_sep, excluded_dirs);
            } else {
                app.scan_manager
                    .start_watching(root_path, mapping, char_sep);
            }
        }

        if !staging_path_str.is_empty() {
            app.scan_manager.start_staging(staging_path_str);
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

    pub fn open_edit_modal(&mut self, item: Arc<MediaItem>) {
        self.modal_state.open_edit(item, &self.autocomplete);
    }

    fn do_save_edit(&mut self) {
        let Some(MediaModalMode::Edit(item)) = &self.modal_state.mode else {
            return;
        };
        let path = item.path.clone();
        let copyright = self.modal_state.copyright.trim().to_owned();
        let artist = self.modal_state.artist.trim().to_owned();
        let characters = self.modal_state.characters.clone();
        let tags = self.modal_state.tags.clone();

        DbService::update_metadata(
            path.clone(),
            copyright.clone(),
            artist.clone(),
            characters.clone(),
            tags.clone(),
        );

        self.apply_metadata_update(&path, copyright, artist, characters, tags);
        self.modal_state.close();
        self.request_stats();
        self.request_autocomplete();
    }

    fn apply_metadata_update(
        &mut self,
        path: &str,
        copyright: String,
        artist: String,
        chars: Vec<String>,
        tags: Vec<String>,
    ) {
        for arc in &mut self.displayed_items {
            if arc.path == path {
                let mut updated = (**arc).clone();
                updated.copyright = copyright;
                updated.artist = artist;
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
        let excluded = self
            .config
            .staging_path
            .as_ref()
            .map(|p| vec![p.to_string_lossy().to_string()])
            .unwrap_or_default();
        self.scan_manager
            .start(self.root_path.clone(), mapping, char_sep, excluded);
    }

    pub fn rescan_staging(&mut self) {
        if let Some(p) = &self.config.staging_path {
            let s = p.to_string_lossy().to_string();
            if !s.is_empty() {
                self.scan_manager.start_staging(s);
            }
        }
    }

    fn request_stats(&mut self) {
        self.stats_rx = Some(DbService::query_stats());
    }
    fn request_autocomplete(&mut self) {
        self.autocomplete_rx = Some(DbService::query_autocomplete());
    }

    fn poll_stats(&mut self, ctx: &Context) {
        let Some(ref rx) = self.stats_rx else { return };
        match rx.try_recv() {
            Ok(s) => {
                self.sidebar_stats = s;
                self.stats_rx = None;
                ctx.request_repaint();
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                self.stats_rx = None;
            }
            _ => {}
        }
    }

    fn poll_autocomplete(&mut self, ctx: &Context) {
        let Some(ref rx) = self.autocomplete_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(d) => {
                self.autocomplete = d;
                self.autocomplete_rx = None;
                ctx.request_repaint();
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                self.autocomplete_rx = None;
            }
            _ => {}
        }
    }

    fn handle_scan_and_watch_events(&mut self, ctx: &Context) {
        let (scan_finished, staging_finished, watch_changed) = self.scan_manager.update();
        if scan_finished || watch_changed {
            self.texture_manager.invalidate_prefetch();
            self.refresh_items();
            self.request_stats();
            self.request_autocomplete();
            ctx.request_repaint();
        }
        if staging_finished {
            self.refresh_staging_items();
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
                Ok((resp_id, items)) => {
                    if resp_id == db_id {
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

    pub fn refresh_staging_items(&mut self) {
        self.staging_rx = Some(DbService::staging_query());
    }

    fn poll_staging(&mut self, ctx: &Context) {
        let Some(ref rx) = self.staging_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(items) => {
                self.staging_items = items;
                self.staging_rx = None;
                ctx.request_repaint();
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                self.staging_rx = None;
            }
            _ => {}
        }
    }

    pub fn open_distribute_modal(&mut self, item: Arc<StagingItem>) {
        self.modal_state.open_distribute(item, &self.autocomplete);
    }

    fn do_distribute(&mut self) {
        let staging_item = match &self.modal_state.mode {
            Some(MediaModalMode::Distribute(item)) => Arc::clone(item),
            _ => return,
        };

        let copyright = self.modal_state.copyright.trim().to_owned();
        let artist = self.modal_state.artist.trim().to_owned();
        let characters = self.modal_state.characters.clone();
        let tags = self.modal_state.tags.clone();
        let video_title = self.modal_state.video_title.trim().to_owned();

        let Some(library_path) = self.config.library_path.clone() else {
            self.modal_state.error = Some("Library path is not configured.".into());
            return;
        };

        let mut dest_dir = library_path.join(&copyright).join(&artist);
        if matches!(staging_item.media_type, MediaType::Video)
            && !self.config.video_subfolder.is_empty()
        {
            dest_dir = dest_dir.join(&self.config.video_subfolder);
        }

        if let Err(e) = fs::create_dir_all(&dest_dir) {
            self.modal_state.error = Some(format!("Could not create destination folder: {e}"));
            return;
        }

        let src_path = Path::new(&staging_item.path);
        let ext = src_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let stem = build_filename_stem(
            &staging_item.media_type,
            &characters,
            &artist,
            &video_title,
            src_path,
            &self.config.character_separator,
        );

        let dest_filename = match resolve_conflict(&dest_dir, &stem, &ext) {
            ResolvedName::Free(name) => name,
            ResolvedName::RenameExisting {
                existing_old,
                existing_new,
                new_file,
            } => {
                let old_p = dest_dir.join(&existing_old);
                let new_p = dest_dir.join(&existing_new);
                if let Err(e) = fs::rename(&old_p, &new_p) {
                    self.modal_state.error = Some(format!("Could not rename existing file: {e}"));
                    return;
                }
                DbService::rename_media_path(
                    old_p.to_string_lossy().to_string(),
                    new_p.to_string_lossy().to_string(),
                    existing_new,
                );
                new_file
            }
            ResolvedName::NextSuffix(name) => name,
        };

        let dest_path = dest_dir.join(&dest_filename);
        if let Err(e) = move_file(src_path, &dest_path) {
            self.modal_state.error = Some(format!("File move failed: {e}"));
            return;
        }

        let dest_path_str = dest_path.to_string_lossy().to_string();
        let modified = fs::metadata(&dest_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let new_item = Arc::new(MediaItem {
            path: dest_path_str,
            name: dest_filename,
            media_type: staging_item.media_type.clone(),
            copyright,
            artist,
            characters,
            tags,
            modified,
        });

        DbService::insert_distributed(Arc::clone(&new_item));
        DbService::staging_delete_by_path(staging_item.path.clone());

        self.staging_items.retain(|i| i.path != staging_item.path);
        self.modal_state.close();

        self.request_autocomplete();
        self.request_stats();
    }
}

impl eframe::App for MediaApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();

        self.poll_db(&ctx);
        self.poll_stats(&ctx);
        self.poll_autocomplete(&ctx);
        self.poll_staging(&ctx);
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
                    .show_inside(ui, |ui| match self.view_mode {
                        ViewMode::Library => sidebar(self, ui),
                        ViewMode::Staging => staging_sidebar(self, ui),
                    });

                components::settings_modal(self, ui);

                {
                    let action = media_modal(self, ui);
                    match action {
                        ModalAction::SaveEdit => self.do_save_edit(),
                        ModalAction::Distribute => self.do_distribute(),
                        ModalAction::Close => self.modal_state.close(),
                        ModalAction::None => {}
                    }
                }

                egui::CentralPanel::default().show_inside(ui, |ui| match self.view_mode {
                    ViewMode::Library => components::grid_layout(self, ui),
                    ViewMode::Staging => staging_view(self, ui),
                });
            });
    }
}
