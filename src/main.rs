use crate::ui::app::MediaApp;

mod core;
mod data;
mod infra;
mod ui;
mod utils;

fn main() {
    let options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "Media Viewer",
        options,
        Box::new(|cc| Ok(Box::new(MediaApp::new(cc)))),
    );
}
