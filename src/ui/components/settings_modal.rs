use crate::ui::app::MediaApp;
use rfd::FileDialog;

pub fn settings_modal(app: &mut MediaApp, ctx: &egui::Context) {
    let mut is_open = app.settings_open.unwrap_or(false);
    if !is_open {
        return;
    }

    egui::Window::new("Настройки")
        .collapsible(false)
        .resizable(false)
        .open(&mut is_open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Путь:");
                ui.label(&app.root_path);

                if ui.button("Выбрать").clicked() {
                    if let Some(folder) = FileDialog::new()
                        .set_directory(&app.root_path)
                        .pick_folder()
                    {
                        app.root_path = folder.to_string_lossy().to_string();
                        app.config.library_path = Some(folder.into());
                        let _ = app.config.save();
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Сканировать").clicked() {
                    app.scan_manager.start(app.root_path.clone());
                }

                if app.scan_manager.is_scanning {
                    ui.spinner();
                }
            });
        });

    app.settings_open = if is_open { Some(true) } else { None };
}
