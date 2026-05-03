use crate::core::models::{MediaType, StagingItem};
use crate::ui::colors::{CARD_BG, DANGER};
use crate::ui::components::card_primitives::{
    draw_card_border, draw_hover_label, draw_hover_tint, draw_info_bar, draw_selection_tint, draw_thumbnail,
    draw_video_badge, CARD_CR, INFO_H,
};
use crate::ui::texture_manager::TextureManager;
use crate::utils::file_helpers::reveal_in_explorer;
use egui::{CursorIcon, Rect, Response, Sense, Ui, Vec2};
use std::sync::Arc;

pub fn staging_card(
    ui: &mut Ui,
    item: &Arc<StagingItem>,
    texture_manager: &mut TextureManager,
    size: f32,
    show_texture: bool,
    is_selected: bool,
    selection_count: usize,
    distribute_target: &mut Option<Arc<StagingItem>>,
    delete_request: &mut Option<Arc<StagingItem>>,
    bulk_delete_request: &mut bool,
    bulk_distribute_request: &mut bool,
    toggle_select: &mut bool,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());

    response.context_menu(|ui| {
        ui.set_min_width(180.0);
        ui.add_space(2.0);

        if ui
            .button("  Open")
            .on_hover_cursor(CursorIcon::PointingHand)
            .clicked()
        {
            let _ = open::that(&item.path);
            ui.close();
        }
        if ui
            .button("  Show in Explorer")
            .on_hover_cursor(CursorIcon::PointingHand)
            .clicked()
        {
            reveal_in_explorer(&item.path);
            ui.close();
        }

        ui.separator();

        if ui
            .button("  Distribute…")
            .on_hover_cursor(CursorIcon::PointingHand)
            .clicked()
        {
            *distribute_target = Some(Arc::clone(item));
            ui.close();
        }

        if is_selected && selection_count > 1 {
            if ui
                .button(format!("  Distribute {selection_count} selected…"))
                .on_hover_cursor(CursorIcon::PointingHand)
                .clicked()
            {
                *bulk_distribute_request = true;
                ui.close();
            }
        }

        ui.separator();

        if ui
            .button("  Copy path")
            .on_hover_cursor(CursorIcon::PointingHand)
            .clicked()
        {
            ui.ctx().copy_text(item.path.clone());
            ui.close();
        }

        ui.separator();

        if is_selected && selection_count > 1 {
            if ui
                .add(egui::Button::new(
                    egui::RichText::new(format!("  Delete {selection_count} selected…"))
                        .color(DANGER),
                ))
                .on_hover_cursor(CursorIcon::PointingHand)
                .clicked()
            {
                *bulk_delete_request = true;
                ui.close();
            }
        }

        if ui
            .add(egui::Button::new(
                egui::RichText::new("  Delete file…").color(DANGER),
            ))
            .on_hover_cursor(CursorIcon::PointingHand)
            .clicked()
        {
            *delete_request = Some(Arc::clone(item));
            ui.close();
        }

        ui.add_space(2.0);
    });

    if response.clicked() {
        if ui.input(|i| i.modifiers.ctrl) {
            *toggle_select = true;
        } else {
            *distribute_target = Some(Arc::clone(item));
        }
    }

    if !ui.is_rect_visible(rect) {
        return response;
    }

    let is_hovered = response.hovered();
    let inner = ui.painter().with_clip_rect(rect);

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

    if is_selected {
        draw_selection_tint(&inner, img_area);
    } else if is_hovered {
        draw_hover_tint(&inner, img_area);
        draw_hover_label(&inner, rect, img_area, "Click to distribute", size);
    }

    if is_hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    draw_info_bar(&inner, rect, &item.name, size);
    draw_card_border(&inner, rect, is_hovered, is_selected);

    response
}
