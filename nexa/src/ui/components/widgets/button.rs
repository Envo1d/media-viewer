use egui::{Color32, CursorIcon, FontId, Sense, Vec2};

const PX: f32 = 14.0;
const PY: f32 = 5.0;

pub fn base_button(
    ui: &mut egui::Ui,
    label: &str,
    fill_normal: Color32,
    fill_hover: Color32,
    fill_press: Color32,
    fill_disabled: Color32,
    text_color: Color32,
    text_color_disabled: Color32,
    enabled: bool,
) -> bool {
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), FontId::proportional(12.0), Color32::WHITE)
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
    let (rect, mut resp) = ui.allocate_exact_size(size, sense);

    if ui.is_rect_visible(rect) {
        let (fill, fg) = if !enabled {
            resp = resp.on_hover_cursor(CursorIcon::NotAllowed);
            (fill_disabled, text_color_disabled)
        } else if resp.is_pointer_button_down_on() {
            (fill_press, text_color)
        } else if resp.hovered() {
            resp = resp.on_hover_cursor(CursorIcon::PointingHand);
            (fill_hover, text_color)
        } else {
            (fill_normal, text_color)
        };

        ui.painter().rect_filled(rect, 6.0, fill);
        ui.painter()
            .galley(rect.min + Vec2::new(PX, PY), galley, fg);
    }

    resp.clicked() && enabled
}
