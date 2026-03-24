use crate::ui::app::MediaApp;

mod core;
mod data;
mod infra;
mod ui;
mod utils;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Oxide View"),
        ..Default::default()
    };

    eframe::run_native(
        "oxide_view_app",
        native_options,
        Box::new(|cc| Ok(Box::new(MediaApp::new(cc)))),
    )
}
