use crate::ui::app::MediaApp;
use crate::ui::colors::{C_INPUT_BG, C_TEXT};
use egui::{CornerRadius, Frame, Margin};

pub fn search_input(app: &mut MediaApp, ui: &mut egui::Ui) {
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
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.search_input)
                            .hint_text("Search...")
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .text_color(C_TEXT),
                    );

                    if response.changed() {
                        app.refresh_items();
                    }
                });

            ui.add_space(12.0);
        },
    );
}
