use crate::ui::app::MediaApp;
use egui::{Align2, Color32, FontId, PointerButton, Sense, Vec2};

fn system_button(
    ui: &mut egui::Ui,
    text: &str,
    hover_color: Color32,
    text_color_hover: Color32,
) -> egui::Response {
    let desired_size = Vec2::new(30.0, ui.available_height());

    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let painter = ui.painter();

        if response.hovered() {
            painter.rect_filled(rect, 0.0, hover_color);
        }

        let text_color = if response.hovered() {
            text_color_hover
        } else {
            visuals.text_color()
        };

        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::monospace(14.0),
            text_color,
        );
    }

    response
}

pub fn custom_title_bar(ui: &mut egui::Ui, app: &mut MediaApp) {
    let height = 32.0;
    let app_icon = app.app_icon.clone();

    ui.horizontal(|ui| {
        ui.set_height(height);
        ui.add_space(8.0);

        if let Some(icon) = app_icon {
            ui.add(egui::Image::from_texture(&icon).fit_to_exact_size(Vec2::splat(20.0)));
            ui.add_space(6.0);
        }

        let rect = ui.available_rect_before_wrap();
        let response = ui.interact(rect, ui.id().with("drag"), Sense::drag());

        if response.dragged_by(PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().button_padding = Vec2::ZERO;

            let close_hover_bg = Color32::from_rgb(210, 45, 57);
            let standard_hover_bg = Color32::from_rgb(29, 29, 30);
            let icon_color = Color32::from_rgb(251, 251, 251);

            if system_button(ui, "❌", close_hover_bg, icon_color).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }

            let is_maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            let max_symbol = if is_maximized { "🗗" } else { "🗖" };
            if system_button(ui, max_symbol, standard_hover_bg, icon_color).clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            }

            if system_button(ui, "-", standard_hover_bg, icon_color).clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }

            ui.add_space(10.0);

            if system_button(ui, "⚙", standard_hover_bg, icon_color).clicked() {
                app.settings_open = Some(true);
            }
        });
    });
}
