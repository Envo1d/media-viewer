use crate::core::models::{MediaItem, MediaType};
use crate::ui::colors::{
    BORDER, CARD_BG, C_BLURPLE, HOVER_TINT, INFO_BG, META_COLOR, NAME_COLOR, PLAY_BG,
};
use crate::ui::texture_manager::TextureManager;
use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

const CR: u8 = 8;

const INFO_H: f32 = 30.0;

#[inline]
fn truncate(s: &str, max_ch: usize) -> std::borrow::Cow<'_, str> {
    if s.chars().count() <= max_ch {
        return std::borrow::Cow::Borrowed(s);
    }
    let body: String = s.chars().take(max_ch.saturating_sub(1)).collect();
    std::borrow::Cow::Owned(format!("{body}…"))
}

pub fn media_card(
    ui: &mut Ui,
    item: &MediaItem,
    texture_manager: &mut TextureManager,
    size: f32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());

    response.context_menu(|ui| {
        ui.set_min_width(190.0);
        ui.add_space(2.0);
        if ui.button("  Open").clicked() {
            let _ = open::that(&item.path);
            ui.close();
        }
        if ui.button("  Show in Explorer").clicked() {
            let _ = std::process::Command::new("explorer")
                .args(["/select,", &item.path])
                .spawn();
            ui.close();
        }
        ui.separator();
        if ui.button("  Copy path").clicked() {
            ui.ctx().copy_text(item.path.clone());
            ui.close();
        }
        if ui.button("  Copy filename").clicked() {
            ui.ctx().copy_text(item.name.clone());
            ui.close();
        }
        ui.add_space(2.0);
    });

    if response.clicked() {
        let _ = open::that(&item.path);
    }

    if !ui.is_rect_visible(rect) {
        return response;
    }

    let is_hovered = response.hovered();

    let inner = ui.painter().with_clip_rect(rect);
    let outer = ui.painter();

    inner.rect_filled(rect, CR, CARD_BG);

    let img_area = Rect::from_min_size(rect.min, Vec2::new(size, size - INFO_H));

    let texture = texture_manager.get(&item.path);
    let tex_sz = texture.size_vec2();

    let scale = (img_area.width() / tex_sz.x).min(img_area.height() / tex_sz.y);
    let img_sz = tex_sz * scale;
    let img_min = img_area.center() - img_sz / 2.0;

    inner.image(
        texture.id(),
        Rect::from_min_size(img_min, img_sz),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );

    if matches!(item.media_type, MediaType::Video) {
        let cc = img_area.center();
        let cr = (size * 0.13).clamp(16.0, 26.0);

        inner.circle_filled(cc, cr + 2.0, Color32::from_black_alpha(40));

        inner.circle_filled(cc, cr, PLAY_BG);

        let ts = cr * 0.42;
        let xoff = ts * 0.12;
        inner.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(cc.x - ts * 0.5 + xoff, cc.y - ts),
                Pos2::new(cc.x - ts * 0.5 + xoff, cc.y + ts),
                Pos2::new(cc.x + ts + xoff, cc.y),
            ],
            Color32::WHITE,
            Stroke::NONE,
        ));
    }

    if is_hovered {
        inner.rect_filled(img_area, 0.0, HOVER_TINT);

        let meta = format!("{} · {}", item.category, item.author);
        let meta_sz = (size * 0.051).clamp(9.0, 11.5);
        let max_meta = ((size * 0.80 / (meta_sz * 0.55)) as usize).max(6);

        let tag_y = img_area.max.y - 6.0;
        if tag_y > img_area.min.y + meta_sz {
            inner.text(
                Pos2::new(rect.center().x, tag_y),
                Align2::CENTER_BOTTOM,
                truncate(&meta, max_meta).as_ref(),
                FontId::proportional(meta_sz),
                META_COLOR,
            );
        }
    }

    let info_rect = Rect::from_min_size(
        Pos2::new(rect.min.x, rect.max.y - INFO_H),
        Vec2::new(rect.width(), INFO_H),
    );

    inner.rect_filled(
        info_rect,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: CR,
            se: CR,
        },
        INFO_BG,
    );

    inner.line_segment(
        [info_rect.left_top(), info_rect.right_top()],
        Stroke::new(1.0, BORDER),
    );

    {
        let font_sz = (size * 0.058).clamp(10.0, 12.5);

        let max_ch = ((size * 0.80 / (font_sz * 0.55)) as usize).max(6);
        inner.text(
            Pos2::new(rect.min.x + 8.0, info_rect.center().y),
            Align2::LEFT_CENTER,
            truncate(&item.name, max_ch).as_ref(),
            FontId::proportional(font_sz),
            NAME_COLOR,
        );
    }

    outer.rect_stroke(
        rect,
        CR,
        Stroke::new(
            if is_hovered { 1.5 } else { 1.0 },
            if is_hovered { C_BLURPLE } else { BORDER },
        ),
        StrokeKind::Outside,
    );

    response
}
