use crate::ui::colors::{C_BLURPLE, C_TEXT_HEADER, C_TEXT_MUTED};
use crate::ui::components::widgets::button::base_button;

pub fn pill_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    base_button(
        ui,
        label,
        C_BLURPLE,
        C_BLURPLE.linear_multiply(0.85),
        C_BLURPLE.linear_multiply(0.70),
        C_BLURPLE.linear_multiply(0.20),
        C_TEXT_HEADER,
        C_TEXT_MUTED,
        enabled,
    )
}
