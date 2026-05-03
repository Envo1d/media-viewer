use crate::core::models::UpdateState;
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    C_BLURPLE, C_PRIMARY_BG, C_SECONDARY_BG, C_TEXT_HEADER, C_TEXT_MUTED, DANGER,
};
use egui::{Color32, FontId, Id, Pos2, Rect, Stroke, StrokeKind, Vec2};

pub fn draw_update_badge(ui: &egui::Ui, btn_rect: Rect, state: &UpdateState) {
    let color = match state {
        UpdateState::Available { .. } | UpdateState::Downloading { .. } => C_BLURPLE,
        UpdateState::ReadyToInstall { .. } => Color32::from_rgb(72, 199, 116),
        UpdateState::Error(_) => DANGER,
        _ => return,
    };

    let dot = Pos2::new(btn_rect.max.x - 6.0, btn_rect.min.y + 6.0);
    let p = ui.painter();
    p.circle_filled(dot, 5.5, C_PRIMARY_BG); // dark halo for contrast
    p.circle_filled(dot, 4.0, color);
}

pub fn update_toast(app: &mut MediaApp, ui: &egui::Ui) {
    let (title, sub, accent) = match &app.update_state {
        UpdateState::Available { version, .. } => (
            format!("Update available — v{version}"),
            "Open Settings → Updates to download".to_owned(),
            C_BLURPLE,
        ),
        UpdateState::ReadyToInstall { version, .. } => (
            format!("v{version} ready to install"),
            "Open Settings → Updates to restart".to_owned(),
            Color32::from_rgb(72, 199, 116),
        ),
        _ => return,
    };

    let ctx = ui.ctx();
    let screen = ctx.content_rect();

    const W: f32 = 288.0;
    const H: f32 = 60.0;
    const MARGIN: f32 = 16.0;
    const CR: f32 = 8.0;

    let origin = Pos2::new(screen.max.x - W - MARGIN, screen.max.y - H - MARGIN);
    let toast_rect = Rect::from_min_size(origin, Vec2::new(W, H));

    let layer = egui::LayerId::new(egui::Order::Tooltip, Id::new("nexa_update_toast"));
    let p = ctx.layer_painter(layer);

    p.rect_filled(toast_rect, CR, C_SECONDARY_BG);
    p.rect_stroke(
        toast_rect,
        CR,
        Stroke::new(1.5, accent),
        StrokeKind::Outside,
    );

    let text_x = toast_rect.min.x + 14.0;

    p.text(
        Pos2::new(text_x, toast_rect.min.y + 20.0),
        egui::Align2::LEFT_CENTER,
        &title,
        FontId::proportional(12.5),
        C_TEXT_HEADER,
    );
    p.text(
        Pos2::new(text_x, toast_rect.min.y + 40.0),
        egui::Align2::LEFT_CENTER,
        &sub,
        FontId::proportional(10.5),
        C_TEXT_MUTED,
    );
}
