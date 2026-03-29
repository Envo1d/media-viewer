use crate::ui::app::MediaApp;

mod core;
mod data;
mod infra;
mod ui;
mod utils;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1550.0, 850.0])
            .with_title("Nexa")
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "nexa_app",
        native_options,
        Box::new(|cc| Ok(Box::new(MediaApp::new(cc)))),
    )
}
