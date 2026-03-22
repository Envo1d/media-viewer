use crate::core::models::{MediaItem, ScanEvent};
use crate::core::scanner::MediaScanner;
use crate::data::db::Database;
use crate::infra::config::AppConfig;
use crate::ui::texture_manager::TextureManager;
use crossbeam_channel::Receiver;
use rfd::FileDialog;
use std::collections::HashSet;

const ITEMS_PER_PAGE: usize = 20;
const MAX_LIVE_ITEMS: usize = 5000;

pub struct MediaApp {
    // core
    db: Database,
    config: AppConfig,
    texture_manager: TextureManager,

    // UI state
    search_input: String,
    root_path: String,
    page: usize,
    settings_open: Option<bool>,

    // scanning
    is_scanning: bool,
    scan_rx: Option<Receiver<ScanEvent>>,

    // transition
    merging_from_db: bool,
    merge_offset: usize,

    // data
    live_items: Vec<MediaItem>,
    displayed_items: Vec<MediaItem>,
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

        Self {
            db: Database::new(),
            config,
            texture_manager: TextureManager::new(&cc.egui_ctx),

            search_input: String::new(),
            root_path,
            page: 0,

            scan_rx: None,
            is_scanning: false,

            live_items: Vec::new(),
            displayed_items: Vec::new(),
            seen_paths: HashSet::new(),

            merging_from_db: false,
            merge_offset: 0,

            settings_open: None,
        }
    }

    fn start_scan(&mut self) {
        let (tx, rx) = crossbeam_channel::unbounded();

        self.scan_rx = Some(rx);
        self.is_scanning = true;

        self.live_items.clear();
        self.displayed_items.clear();
        self.seen_paths.clear();

        self.merging_from_db = false;
        self.merge_offset = 0;

        MediaScanner::start(self.root_path.clone(), tx);
    }

    fn handle_scan_events(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.scan_rx {
            let mut updated = false;

            for event in rx.try_iter() {
                match event {
                    ScanEvent::Item(item) => {
                        if self.seen_paths.insert(item.path.clone()) {
                            self.live_items.push(item.clone());
                            self.displayed_items.push(item);
                            updated = true;

                            if self.live_items.len() > MAX_LIVE_ITEMS {
                                self.live_items.remove(0);
                            }
                        }
                    }

                    ScanEvent::Finished => {
                        self.is_scanning = false;
                        self.merging_from_db = true;
                        self.merge_offset = 0;
                    }
                }
            }

            if updated {
                ctx.request_repaint();
            }
        }
    }

    fn merge_from_db(&mut self, ctx: &egui::Context) {
        if !self.merging_from_db {
            return;
        }

        let batch = self.db.query(100, self.merge_offset);

        if batch.is_empty() {
            self.merging_from_db = false;
            return;
        }

        for item in batch {
            if self.seen_paths.insert(item.path.clone()) {
                self.displayed_items.push(item);
            }
        }

        self.merge_offset += 100;

        ctx.request_repaint();
    }
}

impl eframe::App for MediaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_scan_events(ctx);
        self.merge_from_db(ctx);

        // TOP PANEL
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Настройки").clicked() {
                    self.settings_open = Some(true);
                }
            });

            ui.horizontal(|ui| {
                ui.label("Поиск:");
                ui.text_edit_singleline(&mut self.search_input);
            });
        });

        // SETTINGS MODAL
        if let Some(mut open) = self.settings_open.take() {
            egui::Window::new("Настройки")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Путь:");
                        ui.label(&self.root_path);

                        if ui.button("Выбрать").clicked() {
                            if let Some(folder) = FileDialog::new()
                                .set_directory(&self.root_path)
                                .pick_folder()
                            {
                                self.root_path = folder.to_string_lossy().to_string();
                                self.config.library_path = Some(folder.into());
                                let _ = self.config.save();
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Сканировать").clicked() {
                            self.start_scan();
                            self.page = 0;
                        }

                        if self.is_scanning {
                            ui.spinner();
                        }
                    });
                });

            if open {
                self.settings_open = Some(true);
            } else {
                self.settings_open = None;
            }
        }

        // DATA SOURCE
        let items: Vec<MediaItem> = if self.is_scanning || self.merging_from_db {
            self.displayed_items.clone()
        } else {
            let offset = self.page * ITEMS_PER_PAGE;

            if self.search_input.trim().is_empty() {
                self.db.query(ITEMS_PER_PAGE, offset)
            } else {
                self.db.search(&self.search_input, ITEMS_PER_PAGE, offset)
            }
        };

        // GRID
        egui::CentralPanel::default().show(ctx, |ui| {
            for row in items.chunks(5) {
                ui.horizontal(|ui| {
                    for item in row {
                        ui.group(|ui| {
                            ui.set_max_size(egui::vec2(200.0, 200.0));

                            let texture = self.texture_manager.get(ctx, &item.path);

                            ui.image(&texture);
                            ui.label(&item.name);

                            if ui.button("Открыть").clicked() {
                                let _ = open::that(&item.path);
                            }
                        });
                    }
                });
            }

            ui.separator();

            if !self.is_scanning && !self.merging_from_db {
                ui.horizontal(|ui| {
                    if ui.button("<").clicked() && self.page > 0 {
                        self.page -= 1;
                    }

                    ui.label(format!("Страница {}", self.page + 1));

                    if ui.button(">").clicked() {
                        self.page += 1;
                    }
                });
            }
        });
    }
}
