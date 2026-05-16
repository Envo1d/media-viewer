use crate::ui::colors::{BORDER, C_BLURPLE, C_INPUT_BG};
use egui::{Color32, Id, Pos2, Sense, Stroke, StrokeKind, Vec2};

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

pub fn toggle(ui: &mut egui::Ui, id: Id, value: &mut bool) -> bool {
    const W: f32 = 38.0;
    const H: f32 = 22.0;

    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(W, H), Sense::click());
    
    if resp.hovered() {
        resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    
    if resp.clicked() {
        *value = !*value;
    }

    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool(id, *value);
        let track = Color32::from_rgb(
            lerp_u8(C_INPUT_BG.r(), C_BLURPLE.r(), t),
            lerp_u8(C_INPUT_BG.g(), C_BLURPLE.g(), t),
            lerp_u8(C_INPUT_BG.b(), C_BLURPLE.b(), t),
        );

        let p = ui.painter();
        p.rect_filled(rect, H / 2.0, track);
        p.rect_stroke(rect, H / 2.0, Stroke::new(1.0, BORDER), StrokeKind::Outside);

        let knob_x = rect.min.x + H / 2.0 + t * (W - H);
        p.circle_filled(
            Pos2::new(knob_x, rect.center().y),
            H / 2.0 - 3.0,
            Color32::WHITE,
        );
    }

    resp.clicked()
}
