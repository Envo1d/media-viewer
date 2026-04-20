use crate::ui::colors::{C_HOVER, C_INPUT_BG, C_TEXT, C_TEXT_MUTED};
use egui::{CornerRadius, CursorIcon, Frame, Id, Margin, Sense, TextureHandle, Vec2};

pub struct SearchInputResponse {
    pub changed: bool,
    pub cleared: bool,
}

pub fn search_input(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    close_icon: &TextureHandle,
    id_salt: impl std::hash::Hash,
) -> SearchInputResponse {
    let mut response = SearchInputResponse {
        changed: false,
        cleared: false,
    };

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 36.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            Frame::NONE
                .fill(C_INPUT_BG)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin {
                    left: 10,
                    right: 10,
                    top: 8,
                    bottom: 8,
                })
                .stroke(ui.ctx().global_style().visuals.window_stroke())
                .show(ui, |ui| {
                    let text_resp = ui.add(
                        egui::TextEdit::singleline(value)
                            .hint_text(hint)
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .text_color(C_TEXT),
                    );

                    if text_resp.changed() {
                        response.changed = true;
                    }

                    if !value.is_empty() {
                        let btn_size = 18.0;
                        let rect = text_resp.rect;
                        let btn_rect = egui::Rect::from_center_size(
                            egui::pos2(rect.max.x - btn_size * 0.6, rect.center().y),
                            Vec2::splat(btn_size),
                        );

                        let clear_id = Id::new("search_clear").with(id_salt);
                        let btn_resp = ui.interact(btn_rect, clear_id, Sense::click());

                        if btn_resp.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }

                        if ui.is_rect_visible(btn_rect) {
                            if btn_resp.hovered() {
                                ui.painter().circle_filled(btn_rect.center(), 9.0, C_HOVER);
                            }
                            let tint = if btn_resp.hovered() {
                                C_TEXT
                            } else {
                                C_TEXT_MUTED
                            };
                            let icon_rect =
                                egui::Rect::from_center_size(btn_rect.center(), Vec2::splat(12.0));
                            ui.painter().image(
                                close_icon.id(),
                                icon_rect,
                                egui::Rect::from_min_max(
                                    egui::Pos2::ZERO,
                                    egui::Pos2::new(1.0, 1.0),
                                ),
                                tint,
                            );
                        }

                        if btn_resp.clicked() {
                            value.clear();
                            response.cleared = true;
                        }
                    }
                });

            ui.add_space(12.0);
        },
    );

    response
}
