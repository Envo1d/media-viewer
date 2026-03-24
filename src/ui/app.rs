use crate::core::models::{MediaItem, MediaType, ScanEvent};
use crate::core::scanner::MediaScanner;
use crate::data::db::Database;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::ui::texture_manager::TextureManager;
use crossbeam_channel::Receiver;
use egui::Vec2;
use rfd::FileDialog;
use std::collections::HashSet;
use std::fs;

const MAX_LIVE_ITEMS: usize = 5000;
const MAX_DISPLAYED: usize = 10000;

pub struct MediaApp {
    // core
    db: Database,
    config: AppConfig,
    texture_manager: TextureManager,

    // UI state
    search_input: String,
    root_path: String,
    settings_open: Option<bool>,

    // scanning
    is_scanning: bool,
    scan_rx: Option<Receiver<ScanEvent>>,

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

    fn start_scan(&mut self) {
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
        self.texture_manager.update(ctx);
        self.handle_scan_events(ctx);

        // TOP PANEL
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Настройки").clicked() {
                    self.settings_open = Some(true);
                }
            });

            ui.horizontal(|ui| {
                ui.label("Поиск:");
                if ui.text_edit_singleline(&mut self.search_input).changed() {
                    if !self.is_scanning {
                        self.refresh_items();
                    }
                }
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
        let items = &self.displayed_items;

        // GRID
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            let item_size = 200.0;
            let spacing = 10.0;
            let available_width = ui.available_width() * 0.8;

            let columns = ((available_width + spacing) / (item_size + spacing))
                .floor()
                .max(1.0) as usize;

            let total_width = columns as f32 * item_size + (columns - 1) as f32 * spacing;
            let side_padding = ((ui.available_width() - total_width) / 2.0).max(0.0);

            let row_height = item_size + spacing;
            let total_rows = (items.len() + columns - 1) / columns;

            egui::ScrollArea::vertical()
                .animated(true)
                .wheel_scroll_multiplier(Vec2::new(2.0, 2.0))
                .show_rows(ui, row_height, total_rows, |ui, row_range| {
                    let margin = 2;
                    let prefetch_rows = (row_range.start.saturating_sub(margin)..row_range.start)
                        .chain(row_range.end..(row_range.end + margin).min(total_rows));

                    for p_row in prefetch_rows {
                        for col in 0..columns {
                            let index = p_row * columns + col;
                            if let Some(item) = items.get(index) {
                                self.texture_manager.prefetch(&item.path);
                            }
                        }
                    }

                    ui.add_space(10.0);

                    for row in row_range {
                        ui.horizontal(|ui| {
                            ui.add_space(side_padding);

                            for col in 0..columns {
                                let index = row * columns + col;
                                if index >= items.len() {
                                    break;
                                }

                                let item = &items[index];

                                ui.allocate_ui_with_layout(
                                    egui::vec2(item_size, item_size),
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
                                        let texture = self.texture_manager.get(ctx, &item.path);

                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(item_size, item_size),
                                            egui::Sense::click(),
                                        );

                                        if response.clicked() {
                                            let _ = open::that(&item.path);
                                        }

                                        let painter = ui.painter();
                                        painter.rect_filled(
                                            rect,
                                            4.0,
                                            egui::Color32::from_gray(30),
                                        );

                                        if matches!(item.media_type, MediaType::Video) {
                                            painter.rect_stroke(
                                                rect,
                                                4.0,
                                                egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                                                egui::StrokeKind::Outside,
                                            );
                                        }

                                        let tex_size = texture.size_vec2();
                                        let scale = (item_size / tex_size.x)
                                            .min(item_size / tex_size.y)
                                            .min(1.0);
                                        let img_size = tex_size * scale;
                                        let img_pos = rect.center() - img_size / 2.0;

                                        painter.image(
                                            texture.id(),
                                            egui::Rect::from_min_size(img_pos, img_size),
                                            egui::Rect::from_min_max(
                                                egui::pos2(0.0, 0.0),
                                                egui::pos2(1.0, 1.0),
                                            ),
                                            egui::Color32::WHITE,
                                        );

                                        if response.hovered() {
                                            painter.rect_filled(
                                                rect,
                                                4.0,
                                                egui::Color32::from_black_alpha(160),
                                            );
                                            let galley = ui.painter().layout(
                                                item.name.clone(),
                                                egui::FontId::proportional(14.0),
                                                egui::Color32::WHITE,
                                                rect.width() - 10.0,
                                            );
                                            painter.galley(
                                                rect.center() - galley.size() / 2.0,
                                                galley,
                                                egui::Color32::WHITE,
                                            );
                                        }
                                    },
                                );

                                if col < columns - 1 {
                                    ui.add_space(spacing);
                                }
                            }
                            ui.add_space(side_padding);
                        });
                    }
                    ui.add_space(10.0);
                });
        });
    }
}
