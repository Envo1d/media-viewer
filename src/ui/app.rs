use crate::core::scanner::MediaScanner;
use crate::data::db::Database;
use crate::infra::config::AppConfig;
use crate::ui::texture_manager::TextureManager;
use rfd::FileDialog;

pub struct MediaApp {
    // resources
    db: Database,
    config: AppConfig,
    texture_manager: TextureManager,

    // UI state
    search_input: String,
    root_path: String,
    page: usize,

    // Windows state
    settings_open: Option<bool>,
}

impl Default for MediaApp {
    fn default() -> Self {
        let config = AppConfig::load();
        let db_path = config.database_path.to_string_lossy().to_string();
        let db = Database::new(&db_path);
        let texture_manager = TextureManager::new();
        let root_path = config
            .library_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "S:\\test".to_string());

        Self {
            search_input: String::new(),
            config,
            db,
            settings_open: None,
            texture_manager,
            root_path,
            page: 0,
        }
    }
}

impl MediaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();
        let db_path = config.database_path.to_string_lossy().to_string();
        let db = Database::new(&db_path);
        let texture_manager = TextureManager::new();
        let root_path = config
            .library_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "S:\\test".to_string());

        Self {
            search_input: String::new(),
            config,
            db,
            settings_open: None,
            texture_manager,
            root_path,
            page: 0,
        }
    }
}

impl eframe::App for MediaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        if let Some(mut open) = self.settings_open.take() {
            egui::Window::new("Настройки")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Путь к библиотеке:");
                        ui.label(&self.root_path);

                        if ui.button("Выбрать папку").clicked() {
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

                    if ui.button("Сканировать").clicked() {
                        let mut scanner = MediaScanner::new(&mut self.db);
                        scanner.scan_directory(&self.root_path);
                        self.page = 0;
                    }
                });

            if open {
                self.settings_open = Some(true);
            } else {
                self.settings_open = None;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let items_per_page = 20;
            let offset = self.page * items_per_page;

            let (page_items, total) = if self.search_input.trim().is_empty() {
                (self.db.query(items_per_page, offset), self.db.count())
            } else {
                (
                    self.db.search(&self.search_input, items_per_page, offset),
                    self.db.search_count(&self.search_input),
                )
            };

            let max_page = if total == 0 {
                0
            } else {
                (total - 1) / items_per_page
            };

            for row in page_items.chunks(5) {
                ui.horizontal(|ui| {
                    for item in row {
                        ui.group(|ui| {
                            ui.set_min_size(egui::vec2(120.0, 120.0));

                            let texture = self.texture_manager.get_or_load(ctx, &item.path);
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

            ui.horizontal(|ui| {
                if ui.button("<").clicked() && self.page > 0 {
                    self.page -= 1;
                }

                ui.label(format!("Страница {} / {}", self.page + 1, max_page + 1));

                if ui.button(">").clicked() && self.page < max_page {
                    self.page += 1;
                }
            });
        });
    }
}
