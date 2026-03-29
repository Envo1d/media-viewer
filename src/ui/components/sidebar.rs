use crate::ui::app::MediaApp;
use crate::ui::components::search_input::search_input;

pub fn sidebar(app: &mut MediaApp, ui: &mut egui::Ui) {
    // Search input
    search_input(app, ui);

    ui.add_space(15.0);
}
