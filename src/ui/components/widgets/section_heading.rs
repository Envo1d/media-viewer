use crate::ui::colors::C_TEXT_MUTED;
use egui::RichText;

pub fn section_heading(ui: &mut egui::Ui, label: &str) {
    ui.add_space(16.0);
    ui.label(RichText::new(label).size(10.5).color(C_TEXT_MUTED));
    ui.add_space(4.0);
}
