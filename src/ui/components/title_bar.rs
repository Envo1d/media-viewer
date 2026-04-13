use crate::core::models::ViewMode;
use crate::ui::app::MediaApp;
use crate::ui::colors::{C_BLURPLE, C_TEXT_MUTED, HOVER_CLOSE, HOVER_STANDARD, ICON_IDLE};
use egui::{Color32, CursorIcon, FontId, Image, PointerButton, Sense, Vec2};

const ICON_SIZE: f32 = 12.0;
const BTN_W: f32 = 36.0;

fn chrome_btn(ui: &mut egui::Ui, hover_bg: Color32, icon: &egui::TextureHandle) -> egui::Response {
    let h = ui.available_height();
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(BTN_W, h), Sense::click());

    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(rect, 0.0, hover_bg);
            resp = resp.on_hover_cursor(CursorIcon::PointingHand);
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

fn view_toggle(ui: &mut egui::Ui, current: &mut ViewMode) {
    const W: f32 = 140.0;
    const H: f32 = 22.0;
    const CR: f32 = 5.0;
    const HALF: f32 = W / 2.0;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(W, H), Sense::hover());

    if !ui.is_rect_visible(rect) {
        return;
    }

    ui.painter().rect_filled(
        rect,
        CR,
        Color32::from_rgba_premultiplied(255, 255, 255, 14),
    );

    let library_rect = egui::Rect::from_min_size(rect.min, Vec2::new(HALF, H));
    let staging_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + HALF, rect.min.y),
        Vec2::new(HALF, H),
    );

    let library_resp = ui.interact(library_rect, ui.id().with("view_lib"), Sense::click());
    let staging_resp = ui.interact(staging_rect, ui.id().with("view_stg"), Sense::click());

    let active_rect = if *current == ViewMode::Library {
        library_rect
    } else {
        staging_rect
    };

    ui.painter().rect_filled(active_rect, CR, C_BLURPLE);

    for (label, is_active, resp) in [
        ("Library", *current == ViewMode::Library, &library_resp),
        ("Staging", *current == ViewMode::Staging, &staging_resp),
    ] {
        let col = if is_active {
            Color32::WHITE
        } else {
            C_TEXT_MUTED
        };
        let label_rect = if label == "Library" {
            library_rect
        } else {
            staging_rect
        };
        ui.painter().text(
            label_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.5),
            col,
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
    }

    if library_resp.clicked() {
        *current = ViewMode::Library;
    }
    if staging_resp.clicked() {
        *current = ViewMode::Staging;
    }
}

pub fn title_bar(ui: &mut egui::Ui, app: &mut MediaApp) {
    let app_icon = app.app_icon.clone();
    let icons = app.icons.as_ref().unwrap();

    ui.horizontal(|ui| {
        ui.set_height(32.0);
        ui.add_space(10.0);

        if let Some(icon) = app_icon {
            ui.add(Image::from_texture(&icon).fit_to_exact_size(Vec2::splat(18.0)));
            ui.add_space(6.0);
        }

        view_toggle(ui, &mut app.view_mode);

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
