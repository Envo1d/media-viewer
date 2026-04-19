use crate::ui::app::MediaApp;

mod core;
mod data;
mod infra;
mod ui;
mod utils;

fn main() -> eframe::Result {
    #[cfg(windows)]
    let _singleton = infra::singleton::acquire();

    let app_icon = {
        let png_bytes = include_bytes!("../assets/icons/icon.png");

        let img = image::load_from_memory(png_bytes)
            .expect("assets/icons/icon.png could not be decoded")
            .into_rgba8();

        let (width, height) = img.dimensions();

        std::sync::Arc::new(egui::IconData {
            rgba: img.into_raw(),
            width,
            height,
        })
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1600.0, 900.0])
        .with_title("Nexa")
        .with_decorations(false)
        .with_transparent(true)
        .with_icon(app_icon);

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
