use crate::core::models::MediaItem;
use crate::data::db::Database;
use crate::infra::cache;
use crate::infra::config::AppConfig;
use crate::ui::components;
use crate::ui::scan_manager::ScanManager;
use crate::ui::texture_manager::TextureManager;
use egui::TextureHandle;
use egui_extras::image::load_image_bytes;
use std::fs;

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

    // data
    pub scan_manager: ScanManager,
    pub displayed_items: Vec<MediaItem>,

    pub app_icon: Option<TextureHandle>,
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

        egui::TopBottomPanel::top("custom_bar")
            .frame(egui::Frame::NONE.fill(ctx.style().visuals.window_fill()))
            .show(ctx, |ui| {
                components::custom_title_bar(ui, &self.app_icon);
            });

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
