use crate::ui::app::MediaApp;
use crate::ui::colors::{C_HOVER, C_INPUT_BG, C_TEXT, C_TEXT_MUTED};
use egui::{CornerRadius, CursorIcon, Frame, Margin, Sense, Vec2};
use std::time::Instant;

pub fn search_input(app: &mut MediaApp, ui: &mut egui::Ui) {
    let close_icon = app.icons.as_ref().unwrap().get("close").clone();

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 68.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.add_space(12.0);

            Frame::NONE
                .fill(C_INPUT_BG)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin {
                    left: 10,
                    right: 10,
                    top: 10,
                    bottom: 10,
                })
                .stroke(ui.ctx().global_style().visuals.window_stroke())
                .show(ui, |ui| {
                    let text_resp = ui.add(
                        egui::TextEdit::singleline(&mut app.search_input)
                            .hint_text("Search...")
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .text_color(C_TEXT),
                    );

                    if text_resp.changed() {
                        app.last_input_time = Instant::now();
                    }

                    let visible = !app.search_input.is_empty();

                    if visible {
                        let btn_size = 18.0;

                        let rect = text_resp.rect;
                        let btn_rect = egui::Rect::from_center_size(
                            egui::pos2(rect.max.x - btn_size * 0.6, rect.center().y),
                            Vec2::splat(btn_size),
                        );

                        let resp = ui.interact(btn_rect, ui.id().with("clear_btn"), Sense::click());

                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }

                        if ui.is_rect_visible(btn_rect) {
                            if resp.hovered() {
                                ui.painter().circle_filled(btn_rect.center(), 9.0, C_HOVER);
                            }

                            let tint = if resp.hovered() { C_TEXT } else { C_TEXT_MUTED };

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

                        if resp.clicked() {
                            app.search_input.clear();
                            app.last_input_time = Instant::now();
                            app.field_filter = None;
                            app.refresh_items();
                        }
                    }
                });

            ui.add_space(12.0);
        },
    );
}
