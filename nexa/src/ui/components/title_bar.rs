use crate::core::models::ViewMode;
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    BORDER, C_BLURPLE, C_INPUT_BG, C_TEXT_MUTED, HOVER_CLOSE, HOVER_STANDARD, ICON_IDLE,
};
use crate::ui::components::update_badge::draw_update_badge;
use egui::{Color32, CursorIcon, FontId, Id, Image, PointerButton, Pos2, Rect, Sense, Vec2};

const ICON_SIZE: f32 = 12.0;
const BTN_W: f32 = 36.0;
const TOGGLE_W: f32 = 148.0;
const TOGGLE_H: f32 = 22.0;
const TOGGLE_CR: f32 = TOGGLE_H / 2.0;
const HALF_W: f32 = TOGGLE_W / 2.0;

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

#[inline]
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        lerp_u8(a.r(), b.r(), t),
        lerp_u8(a.g(), b.g(), t),
        lerp_u8(a.b(), b.b(), t),
        lerp_u8(a.a(), b.a(), t),
    )
}

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
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
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
    let id = Id::new("view_mode_toggle");
    let (track_rect, _) = ui.allocate_exact_size(Vec2::new(TOGGLE_W, TOGGLE_H), Sense::hover());

    let lib_rect = Rect::from_min_size(track_rect.min, Vec2::new(HALF_W, TOGGLE_H));
    let stg_rect = Rect::from_min_size(
        Pos2::new(track_rect.min.x + HALF_W, track_rect.min.y),
        Vec2::new(HALF_W, TOGGLE_H),
    );

    let lib_resp = ui.interact(lib_rect, id.with("lib"), Sense::click());
    let stg_resp = ui.interact(stg_rect, id.with("stg"), Sense::click());

    if lib_resp.hovered() || stg_resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    if lib_resp.clicked() {
        *current = ViewMode::Library;
    }
    if stg_resp.clicked() {
        *current = ViewMode::Staging;
    }

    if !ui.is_rect_visible(track_rect) {
        return;
    }

    let t = ui.ctx().animate_bool(id, *current == ViewMode::Staging);

    let p = ui.painter();

    p.rect_filled(track_rect, TOGGLE_CR, C_INPUT_BG);
    p.rect_stroke(
        track_rect,
        TOGGLE_CR,
        egui::Stroke::new(1.0, BORDER),
        egui::StrokeKind::Outside,
    );

    let pill_x = track_rect.min.x + t * HALF_W;
    let pill_rect = Rect::from_min_size(
        Pos2::new(pill_x, track_rect.min.y),
        Vec2::new(HALF_W, TOGGLE_H),
    );

    let pill_inset = Rect::from_min_size(
        pill_rect.min + Vec2::splat(1.0),
        pill_rect.size() - Vec2::splat(2.0),
    );
    p.rect_filled(pill_inset, TOGGLE_CR - 1.0, C_BLURPLE);

    let lib_color = lerp_color(Color32::WHITE, C_TEXT_MUTED, t);
    let stg_color = lerp_color(C_TEXT_MUTED, Color32::WHITE, t);
    let font = FontId::proportional(11.5);

    p.text(
        lib_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Library",
        font.clone(),
        lib_color,
    );
    p.text(
        stg_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Staging",
        font,
        stg_color,
    );
}

pub fn title_bar(ui: &mut egui::Ui, app: &mut MediaApp) {
    let app_icon = app.app_icon.clone();
    let icons = app.icons.as_ref().unwrap();

    ui.horizontal(|ui| {
        ui.set_height(32.0);
        ui.add_space(10.0);

        if let Some(icon) = app_icon {
            ui.add(Image::from_texture(&icon).fit_to_exact_size(Vec2::splat(18.0)));
            ui.add_space(8.0);
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

            let settings_resp = chrome_btn(ui, HOVER_STANDARD, icons.get("settings"));
            draw_update_badge(ui, settings_resp.rect, &app.update_state);
            if settings_resp.clicked() {
                app.settings_open = Some(true);
            }

            ui.add_space(12.0);
        });
    });
}
