use crate::core::models::{MediaItem, MediaType};
use crate::ui::texture_manager::TextureManager;
use egui::{hex_color, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

pub fn media_card(
    ui: &mut Ui,
    item: &MediaItem,
    texture_manager: &mut TextureManager,
    size: f32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());

    if response.clicked() {
        let _ = open::that(&item.path);
    }

    let painter = ui.painter();

    // texture (thumbnail)
    let texture = texture_manager.get(ui.ctx(), &item.path);
    let img_size =
        texture.size_vec2() * (size / texture.size_vec2().x.max(texture.size_vec2().y)).min(1.0);
    let img_pos = rect.center() - img_size / 2.0;

    painter.image(
        texture.id(),
        Rect::from_min_size(img_pos, img_size),
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    // video file indicator
    if matches!(item.media_type, MediaType::Video) {
        let icon_size = 22.0;
        let padding = 6.0;

        let icon_rect = Rect::from_min_size(
            rect.right_bottom() - Vec2::splat(icon_size + padding),
            Vec2::splat(icon_size),
        );

        painter.rect_filled(icon_rect, 6.0, hex_color!("#3D60A3"));

        painter.rect_stroke(
            icon_rect,
            6.0,
            Stroke::new(0.5, Color32::WHITE),
            StrokeKind::Inside,
        );

        painter.text(
            icon_rect.center(),
            egui::Align2::CENTER_CENTER,
            "▶",
            FontId::proportional(14.0),
            Color32::WHITE,
        );
    }

    // hover ui
    if response.hovered() {
        painter.rect_filled(rect, 4.0, Color32::from_black_alpha(160));
        let galley = ui.painter().layout(
            item.name.clone(),
            FontId::proportional(14.0),
            Color32::WHITE,
            rect.width() - 10.0,
        );
        painter.galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
    }

    response
}
