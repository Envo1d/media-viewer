use crate::ui::colors::{
    BORDER, C_BLURPLE, C_HOVER, C_INPUT_BG, C_SECONDARY_BG, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED,
};
use egui::{
    Color32, CornerRadius, FontId, Frame, Id, Margin, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

const BTN_H: f32 = 30.0;
const PAD_X: f32 = 10.0;
const ARROW_ZONE: f32 = 24.0;
const ITEM_H: f32 = 28.0;
const FONT_SZ: f32 = 12.5;

fn draw_chevron(ui: &Ui, centre: Pos2, open: bool) {
    let w = 5.0_f32;
    let h = 3.0_f32;
    let pts = if open {
        vec![
            Pos2::new(centre.x - w, centre.y + h),
            Pos2::new(centre.x + w, centre.y + h),
            Pos2::new(centre.x, centre.y - h),
        ]
    } else {
        vec![
            Pos2::new(centre.x - w, centre.y - h),
            Pos2::new(centre.x + w, centre.y - h),
            Pos2::new(centre.x, centre.y + h),
        ]
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(pts, C_TEXT_MUTED, Stroke::NONE));
}

pub fn combo_box(
    ui: &mut Ui,
    id: Id,
    selected_label: &str,
    options: &[&str],
    width: f32,
) -> Option<usize> {
    let (btn_rect, btn_resp) = ui.allocate_exact_size(Vec2::new(width, BTN_H), Sense::click());

    let is_open = ui
        .ctx()
        .memory(|m| m.data.get_temp::<bool>(id).unwrap_or(false));

    if ui.is_rect_visible(btn_rect) {
        let bg = if is_open || btn_resp.hovered() {
            C_HOVER
        } else {
            C_INPUT_BG
        };

        ui.painter().rect_filled(btn_rect, 6.0, bg);
        ui.painter()
            .rect_stroke(btn_rect, 6.0, Stroke::new(1.0, BORDER), StrokeKind::Outside);

        ui.painter().text(
            Pos2::new(btn_rect.min.x + PAD_X, btn_rect.center().y),
            egui::Align2::LEFT_CENTER,
            selected_label,
            FontId::proportional(FONT_SZ),
            C_TEXT,
        );

        let chevron_cx = btn_rect.max.x - ARROW_ZONE / 2.0;
        draw_chevron(ui, Pos2::new(chevron_cx, btn_rect.center().y), is_open);
    }

    if btn_resp.clicked() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(id, !is_open));
    }

    if !is_open {
        return None;
    }

    let mut result: Option<usize> = None;

    let popup_pos = Pos2::new(btn_rect.min.x, btn_rect.max.y + 2.0);
    let popup_inner_h = options.len() as f32 * ITEM_H;
    let popup_outer_h = popup_inner_h + 8.0;

    let screen_bottom = ui.ctx().content_rect().max.y;
    let popup_y = if popup_pos.y + popup_outer_h > screen_bottom - 8.0 {
        btn_rect.min.y - popup_outer_h - 2.0
    } else {
        popup_pos.y
    };
    let final_popup_pos = Pos2::new(popup_pos.x, popup_y);

    let area_resp = egui::Area::new(id.with("__combo_popup"))
        .fixed_pos(final_popup_pos)
        .order(egui::Order::Tooltip)
        .show(ui.ctx(), |ui| {
            Frame::NONE
                .fill(C_SECONDARY_BG)
                .corner_radius(CornerRadius::same(8))
                .stroke(Stroke::new(1.0, BORDER))
                .inner_margin(Margin::same(4))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 12,
                    spread: 0,
                    color: Color32::from_black_alpha(80),
                })
                .show(ui, |ui| {
                    ui.set_width(width - 8.0);

                    for (i, &label) in options.iter().enumerate() {
                        let item_id = id.with(i);
                        let (item_rect, item_resp) =
                            ui.allocate_exact_size(Vec2::new(width - 8.0, ITEM_H), Sense::click());

                        if ui.is_rect_visible(item_rect) {
                            let bg = if item_resp.hovered() {
                                C_HOVER
                            } else {
                                Color32::TRANSPARENT
                            };
                            ui.painter()
                                .rect_filled(item_rect, CornerRadius::same(5), bg);

                            if item_resp.hovered() {
                                let stripe = Rect::from_min_size(
                                    item_rect.min,
                                    Vec2::new(3.0, item_rect.height()),
                                );
                                ui.painter()
                                    .rect_filled(stripe, CornerRadius::same(2), C_BLURPLE);
                            }

                            let text_color = if item_resp.hovered() {
                                C_TEXT_HEADER
                            } else {
                                C_TEXT
                            };
                            ui.painter().text(
                                Pos2::new(item_rect.min.x + 14.0, item_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                label,
                                FontId::proportional(FONT_SZ),
                                text_color,
                            );
                        }

                        if item_resp.clicked() {
                            result = Some(i);
                        }

                        let _ = item_id; // suppress unused warning
                    }
                });
        });

    let pointer_pos = ui
        .ctx()
        .input(|i| i.pointer.interact_pos())
        .unwrap_or(Pos2::ZERO);
    let any_click = ui.ctx().input(|i| i.pointer.any_click());

    let inside_button = btn_rect.contains(pointer_pos);
    let inside_popup = area_resp.response.rect.contains(pointer_pos);

    if any_click && !inside_button && !inside_popup {
        ui.ctx().memory_mut(|m| m.data.insert_temp(id, false));
    }

    if result.is_some() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(id, false));
    }

    result
}
