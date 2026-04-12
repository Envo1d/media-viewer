use crate::ui::colors::{C_BLURPLE, C_HOVER, C_SELECTED, C_TEXT, C_TEXT_HEADER};
use egui::{Color32, CornerRadius, CursorIcon, FontId, Sense, Vec2};

pub fn sort_row(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let desired = Vec2::new(ui.available_width(), 28.0);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if active {
            C_SELECTED
        } else if response.hovered() {
            C_HOVER
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, CornerRadius::same(5), fill);

        if active {
            let stripe = egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
            ui.painter()
                .rect_filled(stripe, CornerRadius::same(2), C_BLURPLE);
        }

        let text_color = if active { C_TEXT_HEADER } else { C_TEXT };
        ui.painter().text(
            egui::pos2(rect.min.x + 14.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.0),
            text_color,
        );
    }

    if response.hovered() {
        response = response.on_hover_cursor(CursorIcon::PointingHand);
    }

    response.clicked()
}
