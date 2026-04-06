use crate::ui::app::MediaApp;
use crate::ui::colors::{HOVER_CLOSE, HOVER_STANDARD, ICON_IDLE};
use egui::{Color32, Image, PointerButton, Sense, Vec2};

const ICON_SIZE: f32 = 12.0;
const BTN_W: f32 = 36.0;

fn chrome_btn(ui: &mut egui::Ui, hover_bg: Color32, icon: &egui::TextureHandle) -> egui::Response {
    let h = ui.available_height();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(BTN_W, h), Sense::click());

    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(rect, 0.0, hover_bg);
        }

        let tint = if resp.hovered() {
            Color32::WHITE
        } else {
            ICON_IDLE
        };

        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));

        ui.put(
            icon_rect,
            Image::new(icon)
                .fit_to_exact_size(Vec2::splat(ICON_SIZE))
                .tint(tint),
        );
    }

    resp
}

pub fn custom_title_bar(ui: &mut egui::Ui, app: &mut MediaApp) {
    let app_icon = app.app_icon.clone();
    let icons = app.icons.as_ref().unwrap();

    ui.horizontal(|ui| {
        ui.set_height(32.0);
        ui.add_space(10.0);

        if let Some(icon) = app_icon {
            ui.add(Image::from_texture(&icon).fit_to_exact_size(Vec2::splat(18.0)));
            ui.add_space(6.0);
        }

        let drag_rect = ui.available_rect_before_wrap();
        let drag_resp = ui.interact(drag_rect, ui.id().with("drag"), Sense::drag());
        if drag_resp.dragged_by(PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            if chrome_btn(ui, HOVER_CLOSE, icons.get("close")).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }

            ui.add_space(12.0);

            let is_maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            let icon = if is_maximized {
                icons.get("restore")
            } else {
                icons.get("maximize")
            };

            if chrome_btn(ui, HOVER_STANDARD, icon).clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            }

            ui.add_space(12.0);

            if chrome_btn(ui, HOVER_STANDARD, icons.get("minimize")).clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }

            ui.add_space(12.0);

            if chrome_btn(ui, HOVER_STANDARD, icons.get("settings")).clicked() {
                app.settings_open = Some(true);
            }

            ui.add_space(12.0);
        });
    });
}
