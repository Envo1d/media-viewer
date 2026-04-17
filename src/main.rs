use crate::ui::app::MediaApp;

mod core;
mod data;
mod infra;
mod ui;
mod utils;

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1600.0, 900.0])
        .with_title("Nexa")
        .with_decorations(false)
        .with_transparent(true);

    #[cfg(windows)]
    {
        viewport = viewport.with_transparent(false);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "nexa_app",
        native_options,
        Box::new(|cc| Ok(Box::new(MediaApp::new(cc)))),
    )
}
