use crate::ui::colors::{C_BLURPLE, C_TEXT_HEADER, C_TEXT_MUTED};
use egui::{Color32, FontId, Sense, Vec2};

pub fn pill_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    const PX: f32 = 14.0;
    const PY: f32 = 5.0;

    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), FontId::proportional(12.0), C_TEXT_HEADER)
    });

    let size = Vec2::new(
        galley.rect.width() + PX * 2.0,
        galley.rect.height() + PY * 2.0,
    );

    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    if ui.is_rect_visible(rect) {
        let fill = if !enabled {
            Color32::from_rgba_premultiplied(255, 255, 255, 6)
        } else if resp.is_pointer_button_down_on() {
            C_BLURPLE.linear_multiply(0.70)
        } else if resp.hovered() {
            C_BLURPLE.linear_multiply(0.85)
        } else {
            C_BLURPLE
        };

        let text_color = if enabled { C_TEXT_HEADER } else { C_TEXT_MUTED };
        ui.painter().rect_filled(rect, 6.0, fill);
        ui.painter()
            .galley(rect.min + Vec2::new(PX, PY), galley, text_color);
    }

    resp.clicked() && enabled
}
