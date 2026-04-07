use crate::ui::colors::{DANGER, DANGER_HOVER};
use egui::{Color32, FontId, Sense, Vec2};

pub fn danger_button(ui: &mut egui::Ui, label: &str) -> bool {
    const PX: f32 = 14.0;
    const PY: f32 = 5.0;

    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), FontId::proportional(12.0), Color32::WHITE)
    });

    let size = Vec2::new(
        galley.rect.width() + PX * 2.0,
        galley.rect.height() + PY * 2.0,
    );

    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if resp.is_pointer_button_down_on() {
            DANGER.linear_multiply(0.70)
        } else if resp.hovered() {
            DANGER_HOVER
        } else {
            DANGER
        };

        ui.painter().rect_filled(rect, 6.0, fill);
        ui.painter()
            .galley(rect.min + Vec2::new(PX, PY), galley, Color32::WHITE);
    }

    resp.clicked()
}
