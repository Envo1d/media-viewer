use crate::core::models::{MediaItem, MediaType, ScanEvent};
use crate::core::scanner::MediaScanner;
use crate::data::db::Database;
use crate::infra::config::AppConfig;
use crate::ui::texture_manager::TextureManager;
use crossbeam_channel::Receiver;
use rfd::FileDialog;
use std::collections::HashSet;

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
                self.merging_from_db = true;
                self.merge_offset = 0;
            }

            if added > 0 || finished {
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
        self.texture_manager.update(ctx);
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
        let items: &Vec<MediaItem> = if self.is_scanning || self.merging_from_db {
            &self.displayed_items
        } else {
            self.displayed_items = if self.search_input.trim().is_empty() {
                self.db.query(5000, 0)
            } else {
                self.db.search(&self.search_input, 5000, 0)
            };

            &self.displayed_items
        };

        // GRID
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            let item_size = 200.0;
            let spacing = 10.0;
            let max_width = ui.available_width().min(1400.0);

            ui.vertical_centered(|ui| {
                ui.set_max_width(max_width);

                let available_width = ui.available_width();

                let columns = ((available_width + spacing) / (item_size + spacing))
                    .floor()
                    .max(1.0) as usize;

                let total_width = columns as f32 * item_size + (columns - 1) as f32 * spacing;

                let side_padding = ((available_width - total_width) / 2.0).max(0.0);

                let row_height = item_size + spacing;
                let total_rows = (items.len() + columns - 1) / columns;

                egui::ScrollArea::vertical().show_rows(
                    ui,
                    row_height,
                    total_rows,
                    |ui, row_range| {
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

                                            let is_video =
                                                matches!(item.media_type, MediaType::Video);

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

                                            if is_video {
                                                painter.rect_stroke(
                                                    rect,
                                                    4.0,
                                                    egui::Stroke::new(
                                                        2.0,
                                                        egui::Color32::LIGHT_BLUE,
                                                    ),
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
                                                    egui::Pos2::new(0.0, 0.0),
                                                    egui::Pos2::new(1.0, 1.0),
                                                ),
                                                egui::Color32::WHITE,
                                            );

                                            if response.hovered() {
                                                painter.rect_filled(
                                                    rect,
                                                    4.0,
                                                    egui::Color32::from_black_alpha(140),
                                                );

                                                let galley = ui.painter().layout(
                                                    item.name.clone(),
                                                    egui::FontId::proportional(14.0),
                                                    egui::Color32::WHITE,
                                                    rect.width() - 10.0,
                                                );

                                                let text_pos = rect.center() - galley.size() / 2.0;

                                                painter.galley(
                                                    text_pos,
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
                            });
                        }
                    },
                );
            })
        });
    }
}
