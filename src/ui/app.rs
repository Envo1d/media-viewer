use crate::core::models::{
    AutocompleteData, FieldFilter, LibraryStats, MediaFilter, MediaItem, MediaModalMode, MediaType,
    ModalAction, PendingDelete, ReorderAction, ResolvedName, SortOrder, StagingItem, UpdateEvent,
    UpdateState, ViewMode,
};
use crate::data::db_service::DbService;
use crate::data::db_worker::init_db;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::infra::updater::{
    apply_update_and_restart, cleanup_leftover_files, cleanup_staged_downloads, update_staging_dir,
    UpdateWorker,
};
use crate::infra::window_effects::WindowEffects;
use crate::ui::colors::C_PRIMARY_BG;
use crate::ui::components;
use crate::ui::components::media_modal::{media_modal, MediaModalState};
use crate::ui::components::reorder_modal::{do_apply_reorder, reorder_modal, ReorderState};
use crate::ui::components::sidebar::sidebar;
use crate::ui::components::staging_sidebar::staging_sidebar;
use crate::ui::components::staging_view::staging_view;
use crate::ui::components::update_badge::update_toast;
use crate::ui::fonts::setup_fonts;
use crate::ui::icon_registry::IconRegistry;
use crate::ui::scan_manager::ScanManager;
use crate::ui::styles::apply_style;
use crate::ui::texture_manager::TextureManager;
use crate::utils::file_helpers::{move_file, natural_cmp, parse_suffix_number, resolve_conflict};
use crossbeam_channel::Receiver;
use eframe::Frame;
use egui::{Context, Margin, TextureHandle, Ui};
use egui_extras::image::load_image_bytes;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use trash::delete;

const PAGE_SIZE: usize = 200;
const MAX_DISPLAYED_ITEMS: usize = 5000;
const WINDOW_CR: u8 = 12;
const AUTOCOMPLETE_DEBOUNCE: Duration = Duration::from_secs(30);

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
    displayed_index: HashMap<String, usize>,

    // Sidebar statistics
    pub sidebar_stats: LibraryStats,
    stats_rx: Option<Receiver<LibraryStats>>,

    // Autocomplete data for the distribute modal
    pub autocomplete: AutocompleteData,
    autocomplete_rx: Option<Receiver<AutocompleteData>>,
    autocomplete_dirty: bool,
    last_autocomplete_refresh: Instant,

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

    // Pending delete confirmation
    pub pending_delete: Option<PendingDelete>,

    // Staging
    pub staging_items: Vec<Arc<StagingItem>>,
    staging_rx: Option<Receiver<Vec<Arc<StagingItem>>>>,
    pub staging_search: String,
    pub staging_filtered: Vec<Arc<StagingItem>>,
    staging_last_search: String,

    // Windows rounded-window helpers
    pub window_fx: WindowEffects,

    // Reorder modal state (None when closed)
    pub reorder_state: Option<ReorderState>,

    // Multi-selection
    pub selection: std::collections::HashSet<String>,
    pub selection_anchor: Option<String>,
    pub distribute_queue: Vec<Arc<StagingItem>>,

    // Auto-update
    pub update_worker: UpdateWorker,
    pub update_state: UpdateState,
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

        let autocomplete_past = Instant::now()
            .checked_sub(AUTOCOMPLETE_DEBOUNCE + Duration::from_secs(1))
            .unwrap_or_else(Instant::now);

        cleanup_leftover_files();
        cleanup_staged_downloads();

        let update_worker = UpdateWorker::spawn();
        if config.auto_update_check {
            update_worker.check();
        }

        let mut app = Self {
            config: config.clone(),
            texture_manager: TextureManager::new(&cc.egui_ctx),
            search_input: String::new(),
            root_path: root_path.clone(),
            displayed_items: Vec::new(),
            displayed_index: HashMap::new(),
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
            autocomplete_dirty: false,
            last_autocomplete_refresh: autocomplete_past,
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
            pending_delete: None,
            character_separator_input: character_separator,
            video_subfolder_input: video_subfolder,
            window_fx: WindowEffects::new(),
            staging_search: String::new(),
            staging_filtered: Vec::new(),
            staging_last_search: String::new(),
            reorder_state: None,
            selection: std::collections::HashSet::new(),
            selection_anchor: None,
            distribute_queue: Vec::new(),
            update_worker,
            update_state: UpdateState::Idle,
        };

        app.refresh_items();
        app.request_autocomplete();
        app.refresh_staging_items();
        app.request_stats_from_items();

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

    pub fn poll_updater(&mut self, ctx: &Context) {
        let events = self.update_worker.poll();
        if events.is_empty() {
            return;
        }
        for event in events {
            match event {
                UpdateEvent::StateChanged(state) => {
                    self.update_state = state;
                    ctx.request_repaint();
                }
                UpdateEvent::DownloadProgress {
                    bytes_done: ev_bytes_done,
                    total_bytes: ev_total_bytes,
                    progress: ev_progress,
                } => {
                    if let UpdateState::Downloading {
                        ref mut progress,
                        bytes_done: ref mut bd,
                        total_bytes: ref mut tb,
                        ..
                    } = self.update_state
                    {
                        *progress = ev_progress;
                        *bd = ev_bytes_done;
                        *tb = ev_total_bytes;
                    } else if let UpdateState::Available { ref version, .. } =
                        self.update_state.clone()
                    {
                        self.update_state = UpdateState::Downloading {
                            version: version.clone(),
                            progress: ev_progress,
                            bytes_done: ev_bytes_done,
                            total_bytes: ev_total_bytes,
                        };
                    }
                    ctx.request_repaint();
                }
            }
        }
    }

    pub fn start_update_check(&mut self) {
        self.update_state = UpdateState::Checking;
        self.update_worker.check();
    }

    pub fn start_update_download(&mut self) {
        if let UpdateState::Available {
            ref version,
            ref download_url,
            size_bytes,
        } = self.update_state.clone()
        {
            let version = version.clone();
            let url = download_url.clone();
            self.update_state = UpdateState::Downloading {
                version: version.clone(),
                progress: 0.0,
                bytes_done: 0,
                total_bytes: size_bytes,
            };
            self.update_worker
                .download(version, url, update_staging_dir());
        }
    }

    pub fn cancel_update_download(&mut self) {
        self.update_worker.cancel_download();
        self.update_state = UpdateState::Idle;
    }

    pub fn apply_update(&mut self) {
        if let UpdateState::ReadyToInstall {
            ref staged_path, ..
        } = self.update_state.clone()
        {
            if let Err(e) = apply_update_and_restart(staged_path) {
                self.update_state = UpdateState::Error(e);
            }
        }
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

    fn canonical_dir(
        &self,
        library_path: &Path,
        copyright: &str,
        artist: &str,
        media_type: &MediaType,
    ) -> PathBuf {
        let mut dir = library_path.join(copyright).join(artist);
        if matches!(media_type, MediaType::Video) && !self.config.video_subfolder.is_empty() {
            dir = dir.join(&self.config.video_subfolder);
        }
        dir
    }

    fn try_move_library_file(
        &mut self,
        old_path: &str,
        new_copyright: &str,
        new_artist: &str,
        media_type: &MediaType,
    ) -> Result<(String, String), String> {
        let library_path = self
            .config
            .library_path
            .clone()
            .ok_or_else(|| "Library path is not configured.".to_string())?;

        let dest_dir = self.canonical_dir(&library_path, new_copyright, new_artist, media_type);

        let src = Path::new(old_path);

        if src.parent() == Some(dest_dir.as_path()) {
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            return Ok((old_path.to_string(), name));
        }

        fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Could not create destination folder: {e}"))?;

        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_owned();
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let dest_filename = match resolve_conflict(&dest_dir, &stem, &ext) {
            ResolvedName::Free(name) => name,
            ResolvedName::RenameExisting {
                existing_old,
                existing_new,
                new_file,
            } => {
                let old_p = dest_dir.join(&existing_old);
                let new_p = dest_dir.join(&existing_new);
                fs::rename(&old_p, &new_p)
                    .map_err(|e| format!("Could not rename existing file: {e}"))?;
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
        move_file(src, &dest_path).map_err(|e| format!("File move failed: {e}"))?;

        Ok((dest_path.to_string_lossy().into_owned(), dest_filename))
    }

    fn do_save_edit(&mut self) {
        let Some(MediaModalMode::Edit(item)) = &self.modal_state.mode else {
            return;
        };

        let old_path = item.path.clone();
        let old_copyright = item.copyright.clone();
        let old_artist = item.artist.clone();
        let media_type = item.media_type.clone();

        let new_copyright = self.modal_state.copyright.trim().to_owned();
        let new_artist = self.modal_state.artist.trim().to_owned();
        let characters = self.modal_state.characters.clone();
        let tags = self.modal_state.tags.clone();

        let location_changed = new_copyright != old_copyright || new_artist != old_artist;

        let (final_path, final_name) = if location_changed && self.config.library_path.is_some() {
            match self.try_move_library_file(&old_path, &new_copyright, &new_artist, &media_type) {
                Ok(result) => result,
                Err(e) => {
                    self.modal_state.error = Some(e);
                    return;
                }
            }
        } else {
            let name = Path::new(&old_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            (old_path.clone(), name)
        };

        let path_changed = final_path != old_path;

        if path_changed {
            DbService::rename_media_path(old_path.clone(), final_path.clone(), final_name.clone());
        }

        DbService::update_metadata(
            final_path.clone(),
            new_copyright.clone(),
            new_artist.clone(),
            characters.clone(),
            tags.clone(),
        );

        self.apply_full_item_update(
            &old_path,
            final_path,
            final_name,
            new_copyright,
            new_artist,
            characters,
            tags,
            media_type,
        );

        self.modal_state.close();
        self.request_stats_from_items();
        self.request_autocomplete();
    }

    fn apply_full_item_update(
        &mut self,
        old_path: &str,
        new_path: String,
        new_name: String,
        copyright: String,
        artist: String,
        characters: Vec<String>,
        tags: Vec<String>,
        media_type: MediaType,
    ) {
        for arc in &mut self.displayed_items {
            if arc.path == old_path {
                let modified = if new_path != old_path {
                    fs::metadata(&new_path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(arc.modified)
                } else {
                    arc.modified
                };

                *arc = Arc::new(MediaItem {
                    path: new_path,
                    name: new_name,
                    media_type,
                    copyright,
                    artist,
                    characters,
                    tags,
                    modified,
                });
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

    fn request_stats_from_items(&mut self) {
        let n = self.displayed_items.len().min(3);

        if n == 0 {
            self.sidebar_stats = LibraryStats::default();
            return;
        }

        let mut copyrights: Vec<String> = Vec::new();
        let mut artists: Vec<String> = Vec::new();
        let mut tags: Vec<String> = Vec::new();

        for item in &self.displayed_items[..n] {
            if !item.copyright.is_empty() && !copyrights.contains(&item.copyright) {
                copyrights.push(item.copyright.clone());
            }
            if !item.artist.is_empty() && !artists.contains(&item.artist) {
                artists.push(item.artist.clone());
            }
            for tag in &item.tags {
                if !tag.is_empty() && !tags.contains(tag) {
                    tags.push(tag.clone());
                }
            }
        }

        if copyrights.is_empty() && artists.is_empty() && tags.is_empty() {
            self.sidebar_stats = LibraryStats::default();
            return;
        }

        self.stats_rx = Some(DbService::query_stats_for_values(copyrights, artists, tags));
    }

    pub fn request_autocomplete(&mut self) {
        self.autocomplete_dirty = true;
    }

    fn maybe_flush_autocomplete(&mut self) {
        if !self.autocomplete_dirty || self.autocomplete_rx.is_some() {
            return;
        }
        if self.last_autocomplete_refresh.elapsed() < AUTOCOMPLETE_DEBOUNCE {
            return;
        }
        self.autocomplete_rx = Some(DbService::query_autocomplete());
        self.last_autocomplete_refresh = Instant::now();
        self.autocomplete_dirty = false;
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
            if scan_finished {
                self.last_autocomplete_refresh = Instant::now()
                    .checked_sub(AUTOCOMPLETE_DEBOUNCE + Duration::from_secs(1))
                    .unwrap_or_else(Instant::now);
            }
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
        self.displayed_index.clear();
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
        self.clear_selection();
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
                        let is_first_page = self.displayed_items.is_empty();

                        if items.len() < PAGE_SIZE {
                            self.has_more = false;
                        } else {
                            self.page += 1;
                        }

                        self.displayed_items.extend(items);

                        match self.sort {
                            SortOrder::NameAsc => {
                                self.displayed_items
                                    .sort_unstable_by(|a, b| natural_cmp(&a.name, &b.name));
                            }
                            SortOrder::NameDesc => {
                                self.displayed_items
                                    .sort_unstable_by(|a, b| natural_cmp(&b.name, &a.name));
                            }
                            _ => {}
                        }

                        self.displayed_index.clear();
                        for (idx, item) in self.displayed_items.iter().enumerate() {
                            self.displayed_index.insert(item.path.clone(), idx);
                        }

                        if is_first_page {
                            self.request_stats_from_items();
                        }

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

    pub fn rebuild_staging_filtered(&mut self) {
        let q = self.staging_search.trim().to_lowercase();
        self.staging_filtered = if q.is_empty() {
            self.staging_items.clone()
        } else {
            self.staging_items
                .iter()
                .filter(|i| {
                    i.name.to_lowercase().contains(&q) || i.path.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        };
        self.staging_last_search = self.staging_search.clone();
        self.texture_manager.invalidate_prefetch();
    }

    pub fn sync_staging_filter(&mut self) {
        if self.staging_search != self.staging_last_search {
            self.rebuild_staging_filtered();
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
                self.rebuild_staging_filtered();
                self.staging_rx = None;
                ctx.request_repaint();
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                self.staging_rx = None;
            }
            _ => {}
        }
    }

    pub fn do_delete_library(&mut self, item: Arc<MediaItem>) {
        let deleted_path = PathBuf::from(&item.path);

        if let Err(e) = delete(&item.path) {
            eprintln!("[delete] failed to move to trash {}: {e}", item.path);
            return;
        }

        DbService::delete_by_path(item.path.clone());
        self.displayed_items.retain(|i| i.path != item.path);

        let renames = crate::utils::file_helpers::reindex_after_delete(&deleted_path);

        for (old_path, new_path, new_name) in renames {
            DbService::rename_media_path(old_path.clone(), new_path.clone(), new_name.clone());

            for arc in &mut self.displayed_items {
                if arc.path == old_path {
                    *arc = Arc::new(MediaItem {
                        path: new_path.clone(),
                        name: new_name.clone(),
                        media_type: arc.media_type.clone(),
                        copyright: arc.copyright.clone(),
                        artist: arc.artist.clone(),
                        characters: arc.characters.clone(),
                        tags: arc.tags.clone(),
                        modified: arc.modified,
                    });
                    break;
                }
            }
        }

        self.rebuild_display_index();
        self.request_stats_from_items();
    }

    pub fn do_delete_staging(&mut self, item: Arc<StagingItem>) {
        if let Err(e) = delete(&item.path) {
            eprintln!("[delete] failed to move to trash {}: {e}", item.path);
            return;
        }

        DbService::staging_delete_by_path(item.path.clone());
        self.staging_items.retain(|i| i.path != item.path);
        self.rebuild_staging_filtered();
    }

    pub fn rebuild_display_index(&mut self) {
        self.displayed_index.clear();
        for (idx, item) in self.displayed_items.iter().enumerate() {
            self.displayed_index.insert(item.path.clone(), idx);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.selection_anchor = None;
    }

    pub fn do_delete_bulk_library(&mut self, items: Vec<Arc<MediaItem>>) {
        for item in items {
            let deleted_path = PathBuf::from(&item.path);
            if let Err(e) = delete(&item.path) {
                eprintln!("[bulk-delete] failed to trash {}: {e}", item.path);
                continue;
            }
            DbService::delete_by_path(item.path.clone());
            self.displayed_items.retain(|i| i.path != item.path);
            self.selection.remove(&item.path);

            let renames = crate::utils::file_helpers::reindex_after_delete(&deleted_path);
            for (old_path, new_path, new_name) in renames {
                DbService::rename_media_path(old_path.clone(), new_path.clone(), new_name.clone());
                for arc in &mut self.displayed_items {
                    if arc.path == old_path {
                        *arc = Arc::new(MediaItem {
                            path: new_path.clone(),
                            name: new_name.clone(),
                            ..(**arc).clone()
                        });
                        break;
                    }
                }
            }
        }
        self.rebuild_display_index();
        self.request_stats_from_items();
        self.clear_selection();
    }

    pub fn do_delete_bulk_staging(&mut self, items: Vec<Arc<StagingItem>>) {
        for item in items {
            if let Err(e) = delete(&item.path) {
                eprintln!("[bulk-delete] failed to trash {}: {e}", item.path);
                continue;
            }
            DbService::staging_delete_by_path(item.path.clone());
            self.staging_items.retain(|i| i.path != item.path);
            self.selection.remove(&item.path);
        }
        self.rebuild_staging_filtered();
        self.clear_selection();
    }

    pub fn open_reorder_modal(&mut self, item: Arc<MediaItem>) {
        let path = Path::new(&item.path);
        let dir = path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        let base_stem = if let Some((base, _)) = parse_suffix_number(stem) {
            base
        } else {
            stem.to_owned()
        };
        self.reorder_state = Some(ReorderState::new(base_stem, ext, dir));
    }

    pub fn open_distribute_modal(&mut self, item: Arc<StagingItem>) {
        self.modal_state.open_distribute(item, &self.autocomplete);
    }

    pub fn open_distribute_queue(&mut self, mut items: Vec<Arc<StagingItem>>) {
        if items.is_empty() {
            return;
        }
        let first = items.remove(0);
        self.distribute_queue = items;
        self.open_distribute_modal(first);
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

        let dest_dir =
            self.canonical_dir(&library_path, &copyright, &artist, &staging_item.media_type);

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

        let stem = {
            use crate::utils::file_helpers::build_filename_stem;
            build_filename_stem(
                &staging_item.media_type,
                &characters,
                &artist,
                &video_title,
                src_path,
                &self.config.character_separator,
            )
        };

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
        self.rebuild_staging_filtered();

        self.request_autocomplete();
        self.request_stats_from_items();

        if !self.distribute_queue.is_empty() {
            let next_item = self.distribute_queue.remove(0);

            let saved_copyright = self.modal_state.copyright.clone();
            let saved_artist = self.modal_state.artist.clone();
            let saved_characters = self.modal_state.characters.clone();
            let saved_tags = self.modal_state.tags.clone();

            self.modal_state
                .open_distribute(next_item, &self.autocomplete);

            self.modal_state.copyright = saved_copyright;
            self.modal_state.artist = saved_artist;
            self.modal_state.characters = saved_characters;
            self.modal_state.tags = saved_tags;
        } else {
            self.modal_state.close();
        }
    }
}

impl eframe::App for MediaApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        self.window_fx.apply();

        let ctx = ui.ctx().clone();

        self.poll_db(&ctx);
        self.poll_stats(&ctx);
        self.poll_autocomplete(&ctx);
        self.poll_staging(&ctx);
        self.poll_updater(&ctx);
        self.handle_search_input(&ctx);
        self.maybe_flush_autocomplete();
        self.texture_manager.update(&ctx);
        self.handle_scan_and_watch_events(&ctx);
        self.sync_staging_filter();

        let window_frame = egui::Frame::NONE
            .fill(C_PRIMARY_BG)
            .stroke(ctx.global_style().visuals.window_stroke());

        egui::CentralPanel::default()
            .frame(window_frame)
            .show_inside(ui, |ui| {
                egui::Panel::top("custom_bar")
                    .frame(egui::Frame::NONE.corner_radius(egui::CornerRadius {
                        nw: WINDOW_CR,
                        ne: WINDOW_CR,
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
                        ModalAction::Close => {
                            self.modal_state.close();
                            self.distribute_queue.clear();
                        }
                        ModalAction::None => {}
                    }
                }

                components::delete_confirm_modal(self, ui);

                {
                    let ro_action = reorder_modal(self, ui);
                    match ro_action {
                        ReorderAction::Apply => do_apply_reorder(self),
                        ReorderAction::Close => self.reorder_state = None,
                        ReorderAction::None => {}
                    }
                }

                update_toast(self, ui);

                egui::CentralPanel::default().show_inside(ui, |ui| match self.view_mode {
                    ViewMode::Library => components::media_view(self, ui),
                    ViewMode::Staging => staging_view(self, ui),
                });
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
