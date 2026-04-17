use crate::core::models::MediaType;
use crate::ui::colors::{
    BORDER, CARD_BG, C_BLURPLE, HOVER_TINT, INFO_BG, META_COLOR, NAME_COLOR, PLAY_BG,
};
use crate::utils::truncate;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

pub const CARD_CR: u8 = 8;
pub const INFO_H: f32 = 30.0;

pub fn draw_thumbnail(
    painter: &egui::Painter,
    img_area: Rect,
    media_type: &MediaType,
    texture: Option<&egui::TextureHandle>,
) {
    if let Some(tex) = texture {
        let tex_sz = tex.size_vec2();
        let scale = (img_area.width() / tex_sz.x).min(img_area.height() / tex_sz.y);
        let img_sz = tex_sz * scale;
        let img_min = img_area.center() - img_sz / 2.0;
        painter.image(
            tex.id(),
            Rect::from_min_size(img_min, img_sz),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.rect_filled(img_area, 8.0, CARD_BG);
        let icon = match media_type {
            MediaType::Image => "🖼",
            MediaType::Video => "🎬",
        };
        painter.text(
            img_area.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(28.0),
            Color32::from_gray(140),
        );
    }
}

pub fn draw_video_badge(painter: &egui::Painter, img_area: Rect, card_size: f32) {
    let cc = img_area.center();
    let cr = (card_size * 0.13).clamp(16.0, 26.0);
    painter.circle_filled(cc, cr + 2.0, Color32::from_black_alpha(40));
    painter.circle_filled(cc, cr, PLAY_BG);
    let ts = cr * 0.42;
    let xoff = ts * 0.12;
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(cc.x - ts * 0.5 + xoff, cc.y - ts),
            Pos2::new(cc.x - ts * 0.5 + xoff, cc.y + ts),
            Pos2::new(cc.x + ts + xoff, cc.y),
        ],
        Color32::WHITE,
        Stroke::NONE,
    ));
}

pub fn draw_hover_tint(painter: &egui::Painter, img_area: Rect) {
    painter.rect_filled(img_area, 0.0, HOVER_TINT);
}

pub fn draw_hover_label(
    painter: &egui::Painter,
    card_rect: Rect,
    img_area: Rect,
    text: &str,
    card_size: f32,
) {
    let hint_sz = (card_size * 0.051).clamp(9.0, 11.5);
    let tag_y = img_area.max.y - 6.0;
    if tag_y > img_area.min.y + hint_sz {
        painter.text(
            Pos2::new(card_rect.center().x, tag_y),
            Align2::CENTER_BOTTOM,
            text,
            FontId::proportional(hint_sz),
            META_COLOR,
        );
    }
}

pub fn draw_info_bar(painter: &egui::Painter, card_rect: Rect, name: &str, card_size: f32) {
    let info_rect = Rect::from_min_size(
        Pos2::new(card_rect.min.x, card_rect.max.y - INFO_H),
        Vec2::new(card_rect.width(), INFO_H),
    );

    painter.rect_filled(
        info_rect,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: CARD_CR,
            se: CARD_CR,
        },
        INFO_BG,
    );
    painter.line_segment(
        [info_rect.left_top(), info_rect.right_top()],
        Stroke::new(1.0, BORDER),
    );

    let font_sz = (card_size * 0.058).clamp(10.0, 12.5);
    let max_ch = ((card_size * 0.80 / (font_sz * 0.55)) as usize).max(6);
    painter.text(
        Pos2::new(card_rect.min.x + 8.0, info_rect.center().y),
        Align2::LEFT_CENTER,
        truncate(name, max_ch).as_ref(),
        FontId::proportional(font_sz),
        NAME_COLOR,
    );
}

pub fn draw_card_border(outer: &egui::Painter, rect: Rect, is_hovered: bool) {
    outer.rect_stroke(
        rect,
        CARD_CR,
        Stroke::new(
            if is_hovered { 1.5 } else { 1.0 },
            if is_hovered { C_BLURPLE } else { BORDER },
        ),
        StrokeKind::Outside,
    );
}
