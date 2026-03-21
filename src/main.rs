use crate::ui::App;

mod db;
mod db_migrations;
mod models;
mod ui;
mod utils;

fn main() {
    let options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "Media Viewer",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    );
}
