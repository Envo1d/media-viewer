use crate::core::models::{MediaItem, MediaType};
use crate::ui::colors::{CARD_BG, DANGER};
use crate::ui::components::card_primitives::{
    draw_card_border, draw_hover_label, draw_hover_tint, draw_info_bar, draw_thumbnail, draw_video_badge,
    CARD_CR, INFO_H,
};
use crate::ui::texture_manager::TextureManager;
use crate::utils::file_helpers::reveal_in_explorer;
use crate::utils::truncate;
use egui::{CursorIcon, Rect, Response, Sense, Ui, Vec2};
use std::sync::Arc;

fn hover_meta(item: &MediaItem) -> String {
    if !item.characters.is_empty() {
        const MAX: usize = 3;
        let shown = &item.characters[..item.characters.len().min(MAX)];
        let mut s = shown.join(" · ");
        if item.characters.len() > MAX {
            s.push_str(&format!(" +{}", item.characters.len() - MAX));
        }
        return s;
    }
    format!("{} · {}", item.copyright, item.artist)
}

pub fn media_card(
    ui: &mut Ui,
    item: &Arc<MediaItem>,
    texture_manager: &mut TextureManager,
    size: f32,
    show_texture: bool,
    edit_target: &mut Option<Arc<MediaItem>>,
    delete_request: &mut Option<Arc<MediaItem>>,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());

    response.context_menu(|ui| {
        ui.set_min_width(190.0);
        ui.add_space(2.0);

        if ui.button("  Open").on_hover_cursor(CursorIcon::PointingHand).clicked() {
            let _ = open::that(&item.path);
            ui.close();
        }
        if ui.button("  Show in Explorer").on_hover_cursor(CursorIcon::PointingHand).clicked() {
            reveal_in_explorer(&item.path);
            ui.close();
        }

        ui.separator();

        if ui.button("  Edit metadata").on_hover_cursor(CursorIcon::PointingHand).clicked() {
            *edit_target = Some(Arc::clone(item));
            ui.close();
        }

        ui.separator();

        if ui.button("  Copy path").on_hover_cursor(CursorIcon::PointingHand).clicked() {
            ui.ctx().copy_text(item.path.clone());
            ui.close();
        }
        if ui.button("  Copy filename").on_hover_cursor(CursorIcon::PointingHand).clicked() {
            ui.ctx().copy_text(item.name.clone());
            ui.close();
        }

        ui.separator();

        if ui
            .add(egui::Button::new(
                egui::RichText::new("  Delete file…").color(DANGER),
            )).on_hover_cursor(CursorIcon::PointingHand)
            .clicked()
        {
            *delete_request = Some(Arc::clone(item));
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

    inner.rect_filled(rect, CARD_CR, CARD_BG);

    let img_area = Rect::from_min_size(rect.min, Vec2::new(size, size - INFO_H));

    let texture = if show_texture {
        Some(texture_manager.get(&item.path))
    } else {
        None
    };
    draw_thumbnail(&inner, img_area, &item.media_type, texture.as_ref());

    if matches!(item.media_type, MediaType::Video) {
        draw_video_badge(&inner, img_area, size);
    }

    if is_hovered {
        draw_hover_tint(&inner, img_area);
        let meta = hover_meta(item);
        let meta_sz = (size * 0.051).clamp(9.0, 11.5);
        let max_ch = ((size * 0.80 / (meta_sz * 0.55)) as usize).max(6);
        draw_hover_label(
            &inner,
            rect,
            img_area,
            truncate(&meta, max_ch).as_ref(),
            size,
        );
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    draw_info_bar(&inner, rect, &item.name, size);
    draw_card_border(&outer, rect, is_hovered);

    response
}
