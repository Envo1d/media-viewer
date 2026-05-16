use crate::ui::colors::{BORDER, SECTION_BG};
use egui::{CornerRadius, Frame, Margin, Sense, Vec2};

const SECTION_CR: u8 = 10;
const ROW_H: f32 = 54.0;

pub fn section_row(
    ui: &mut egui::Ui,
    is_first: bool,
    is_last: bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    let cr = CornerRadius {
        nw: if is_first { SECTION_CR } else { 0 },
        ne: if is_first { SECTION_CR } else { 0 },
        sw: if is_last { SECTION_CR } else { 0 },
        se: if is_last { SECTION_CR } else { 0 },
    };

    Frame::NONE
        .fill(SECTION_BG)
        .corner_radius(cr)
        .inner_margin(Margin::symmetric(14, 0))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(ui.available_width(), ROW_H));
            ui.horizontal(|ui| {
                ui.set_min_height(ROW_H);
                content(ui);
            });
        });

    if !is_last {
        let (sep, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(sep, 0.0, BORDER);
    }
}
