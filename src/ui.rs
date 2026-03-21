use crate::db::Database;
use crate::models::{MediaItem, MediaType};
use crate::utils::cache::{get_cache_dir, preview_cache_path};
use crate::utils::config::AppConfig;
use egui::TextureHandle;
use image::{DynamicImage, GenericImage, Rgba, RgbaImage};
use rfd::FileDialog;
use std::collections::HashMap;
use walkdir::WalkDir;

pub(crate) struct App {
    root_path: String,
    search_input: String,
    page: usize,
    db: Database,
    config: AppConfig,
    show_settings: Option<bool>,
    texture_cache: HashMap<String, TextureHandle>,
}

impl Default for App {
    fn default() -> Self {
        let config = AppConfig::load();
        let db_path = config.database_path.to_string_lossy().to_string();
        let db = Database::new(&db_path);
        Self {
            root_path: String::from("S:\\test"),
            search_input: String::new(),
            page: 0,
            config,
            db,
            show_settings: None,
            texture_cache: HashMap::new(),
        }
    }
}

impl App {
    pub(crate) fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();

        let root_path = config
            .library_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "S:\\test".to_string());

        let db_path = config.database_path.to_string_lossy().to_string();
        let db = Database::new(&db_path);

        Self {
            config,
            root_path,
            search_input: String::new(),
            page: 0,
            db,
            show_settings: None,
            texture_cache: HashMap::new(),
        }
    }

    fn get_texture(&mut self, ctx: &egui::Context, path: &str) -> egui::TextureHandle {
        if let Some(tex) = self.texture_cache.get(path) {
            return tex.clone();
        }

        let cache_dir = get_cache_dir();
        let cache_path = preview_cache_path(&cache_dir, path);

        let final_img: RgbaImage = if cache_path.exists() {
            image::open(&cache_path)
                .unwrap_or_else(|_| DynamicImage::from(RgbaImage::new(120, 120)))
                .to_rgba8()
        } else {
            let img = image::open(path)
                .unwrap_or_else(|_| DynamicImage::from(RgbaImage::new(120, 120)))
                .to_rgba8();

            let target_size = 120;
            let scaled = image::imageops::thumbnail(&img, target_size, target_size);

            let mut canvas =
                RgbaImage::from_pixel(target_size, target_size, Rgba([200, 200, 200, 255]));
            let x_offset = (target_size - scaled.width()) / 2;
            let y_offset = (target_size - scaled.height()) / 2;
            canvas.copy_from(&scaled, x_offset, y_offset).unwrap();

            let _ = canvas.save(&cache_path);

            canvas
        };

        let pixels: Vec<_> = final_img.pixels().flat_map(|p| p.0).collect();
        let size = [final_img.width() as usize, final_img.height() as usize];
        let texture = ctx.load_texture(
            path,
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            Default::default(),
        );

        self.texture_cache.insert(path.to_string(), texture.clone());
        texture
    }

    fn scan(&mut self) {
        use std::fs;

        let mut seen_paths = Vec::new();

        for entry in WalkDir::new(&self.root_path)
            .min_depth(3)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let media_type = match ext.as_str() {
                "mp4" | "mkv" | "avi" => MediaType::Video,
                "jpg" | "png" | "jpeg" | "gif" => MediaType::Image,
                _ => continue,
            };

            let metadata = fs::metadata(path).unwrap();
            let modified = metadata
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            let rel = path.strip_prefix(&self.root_path).unwrap();

            let parts: Vec<_> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect();

            if parts.len() < 3 {
                continue;
            }

            let item = MediaItem {
                path: path.to_string_lossy().to_string(),
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                media_type,
                category: parts[0].to_string(),
                author: parts[1].to_string(),
                modified,
            };

            seen_paths.push(item.path.clone());

            self.db.upsert(&item);
        }

        self.db.delete_missing(&seen_paths);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Настройки").clicked() {
                    self.show_settings = Some(true);
                }
            });

            ui.horizontal(|ui| {
                ui.label("Поиск:");
                ui.text_edit_singleline(&mut self.search_input);
            });
        });

        if let Some(mut open) = self.show_settings.take() {
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
                        self.scan();
                        self.page = 0;
                    }
                });

            if open {
                self.show_settings = Some(true);
            } else {
                self.show_settings = None;
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

                            let texture = self.get_texture(ctx, &item.path);
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
