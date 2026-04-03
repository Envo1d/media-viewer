use crate::ui::app::MediaApp;
use egui::RichText;
use rfd::FileDialog;

fn dir_size_mb(path: &std::path::Path) -> f64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0.0;
    };
    let bytes: u64 = entries
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum();
    bytes as f64 / (1024.0 * 1024.0)
}

pub fn settings_modal(app: &mut MediaApp, ui: &egui::Ui) {
    let ctx = ui.ctx();

    let mut is_open = app.settings_open.unwrap_or(false);
    if !is_open {
        return;
    }

    egui::Window::new("Settings")
        .collapsible(false)
        .resizable(false)
        .min_width(380.0)
        .open(&mut is_open)
        .show(ctx, |ui| {
            ui.label(RichText::new("Library path").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let path_text = if app.root_path.is_empty() {
                    "No folder selected".to_string()
                } else {
                    app.root_path.clone()
                };
                ui.label(&path_text);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Choose…").clicked() {
                        let start = if app.root_path.is_empty() {
                            std::path::PathBuf::from("/")
                        } else {
                            std::path::PathBuf::from(&app.root_path)
                        };

                        if let Some(folder) = FileDialog::new().set_directory(start).pick_folder() {
                            app.root_path = folder.to_string_lossy().to_string();
                            app.config.library_path = Some(folder.into());
                            let _ = app.config.save();
                        }
                    }
                });
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            ui.label(RichText::new("Scan").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let scanning = app.scan_manager.is_scanning;

                ui.add_enabled_ui(!scanning && !app.root_path.is_empty(), |ui| {
                    if ui.button("Scan now").clicked() {
                        app.scan_manager.start(app.root_path.clone());
                    }
                });

                if scanning {
                    ui.spinner();
                    ui.label(
                        RichText::new(format!(
                            "  {} files indexed…",
                            app.scan_manager.files_scanned
                        ))
                        .size(12.0),
                    );
                }
            });

            ui.add_space(8.0);

            let mut auto_scan = app.config.auto_scan;
            if ui
                .checkbox(&mut auto_scan, "Scan automatically on startup")
                .changed()
            {
                app.config.auto_scan = auto_scan;
                let _ = app.config.save();
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            ui.label(RichText::new("Thumbnail cache").strong());
            ui.add_space(4.0);

            let cache_dir = crate::infra::config::AppConfig::get_cache_dir();
            let cache_size = dir_size_mb(&cache_dir);

            ui.label(format!("Location: {}", cache_dir.display()));
            ui.label(format!("Size: {:.1} MB", cache_size));

            ui.add_space(6.0);

            if ui.button("Clear cache").clicked() {
                if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                    for entry in entries.flatten() {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        });

    app.settings_open = if is_open { Some(true) } else { None };
}
