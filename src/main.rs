use crate::ui::app::MediaApp;
use std::sync::Arc;

mod core;
mod data;
mod infra;
mod ui;
mod utils;

fn main() -> eframe::Result {
    let icon_data = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("Не удалось загрузить иконку окна");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Nexa")
            .with_icon(Arc::new(icon_data)),
        ..Default::default()
    };

    eframe::run_native(
        "nexa_app",
        native_options,
        Box::new(|cc| Ok(Box::new(MediaApp::new(cc)))),
    )
}
