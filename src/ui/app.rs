use crate::core::models::MediaItem;
use crate::data::db::Database;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::ui::colors::C_PRIMARY_BG;
use crate::ui::components;
use crate::ui::components::sidebar::sidebar;
use crate::ui::fonts::setup_fonts;
use crate::ui::scan_manager::ScanManager;
use crate::ui::styles::apply_style;
use crate::ui::texture_manager::TextureManager;
use eframe::Frame;
use egui::{Margin, TextureHandle, Ui};
use egui_extras::image::load_image_bytes;
use std::fs;

const MAX_DISPLAYED: usize = 10000;

pub struct MediaApp {
    // core
    db: Database,
    pub config: AppConfig,
    pub texture_manager: TextureManager,

    // UI state
    pub search_input: String,
    pub root_path: String,
    pub settings_open: Option<bool>,

    // data
    pub scan_manager: ScanManager,
    pub displayed_items: Vec<MediaItem>,

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

        let mut app = Self {
            db: Database::new(),
            config,
            texture_manager: TextureManager::new(&cc.egui_ctx),
            search_input: String::new(),
            root_path,
            displayed_items: Vec::new(),
            settings_open: None,
            scan_manager: ScanManager::new(),
            app_icon,
        };

        app.refresh_items();

        app
    }

    fn handle_scan_events(&mut self, ctx: &egui::Context) {
        let (new_items, finished) = self.scan_manager.update();

        if !new_items.is_empty() {
            self.displayed_items.extend(new_items);

            if self.displayed_items.len() > MAX_DISPLAYED {
                let to_remove = self.displayed_items.len() - MAX_DISPLAYED;
                self.displayed_items.drain(0..to_remove);
            }

            ctx.request_repaint();
        }

        if finished {
            self.refresh_items();
            ctx.request_repaint();
        }
    }

    pub fn refresh_items(&mut self) {
        const PAGE_SIZE: usize = 500;
        self.displayed_items = if self.search_input.trim().is_empty() {
            self.db.query(PAGE_SIZE, 0)
        } else {
            self.db.search(&self.search_input, PAGE_SIZE, 0)
        };
    }
}

impl eframe::App for MediaApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.clone();

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
