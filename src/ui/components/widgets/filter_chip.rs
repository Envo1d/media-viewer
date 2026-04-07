use crate::ui::colors::{C_BLURPLE, C_HOVER, C_TEXT, C_TEXT_HEADER};
use egui::{Color32, CornerRadius, FontId, Sense, Vec2};

pub fn filter_chip(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let desired = Vec2::new(ui.available_width(), 30.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if active {
            C_BLURPLE
        } else if response.hovered() {
            C_HOVER
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);

        let text_color = if active { C_TEXT_HEADER } else { C_TEXT };
        ui.painter().text(
            egui::pos2(rect.min.x + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.0),
            text_color,
        );
    }

    response.clicked()
}
