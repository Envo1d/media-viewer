use crate::core::models::{MediaItem, MediaType};
use crate::ui::texture_manager::TextureManager;
use egui::{Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

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

    // background
    painter.rect_filled(rect, 4.0, Color32::from_gray(30));

    // video file indicator
    if matches!(item.media_type, MediaType::Video) {
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(2.0, Color32::LIGHT_BLUE),
            StrokeKind::Outside,
        );
    }

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
