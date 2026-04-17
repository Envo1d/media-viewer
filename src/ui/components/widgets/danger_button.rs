use crate::ui::colors::{DANGER, DANGER_HOVER};
use crate::ui::components::widgets::button::base_button;
use egui::Color32;

pub fn danger_button(ui: &mut egui::Ui, label: &str) -> bool {
    base_button(
        ui,
        label,
        DANGER,
        DANGER_HOVER,
        DANGER.linear_multiply(0.70),
        DANGER,
        Color32::WHITE,
        Color32::WHITE,
        true,
    )
}
