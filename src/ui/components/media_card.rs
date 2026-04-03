use crate::core::models::{MediaItem, MediaType};
use crate::ui::colors::{ACCENT_BLUE, C_TEXT_MUTED, TEXT_HIGHLIGHT, TEXT_LIGHT};
use crate::ui::texture_manager::TextureManager;
use egui::{Align2, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

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

    response.context_menu(|ui| {
        ui.set_min_width(180.0);

        if ui.button("  Open").clicked() {
            let _ = open::that(&item.path);
            ui.close();
        }

        if ui.button("  Open folder").clicked() {
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer")
                    .arg("/select,")
                    .arg(&item.path)
                    .spawn();
            }
            #[cfg(not(target_os = "windows"))]
            {
                if let Some(parent) = std::path::Path::new(&item.path).parent() {
                    let _ = open::that(parent);
                }
            }
            ui.close();
        }

        ui.separator();

        if ui.button("  Copy path").clicked() {
            ui.ctx().copy_text(item.path.clone());
            ui.close();
        }

        if ui.button("  Copy name").clicked() {
            ui.ctx().copy_text(item.name.clone());
            ui.close();
        }
    });

    let painter = ui.painter();

    let texture = texture_manager.get(&item.path);
    let tex_size = texture.size_vec2();

    let scale = (size / tex_size.x.max(tex_size.y)).min(1.0);
    let img_size = tex_size * scale;
    let img_pos = rect.center() - img_size / 2.0;

    painter.image(
        texture.id(),
        Rect::from_min_size(img_pos, img_size),
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    // Video badge
    if matches!(item.media_type, MediaType::Video) {
        let icon_size = 22.0;
        let padding = 6.0;
        let icon_rect = Rect::from_min_size(
            rect.right_bottom() - Vec2::splat(icon_size + padding),
            Vec2::splat(icon_size),
        );

        painter.rect_filled(icon_rect, 6.0, ACCENT_BLUE);
        painter.rect_stroke(
            icon_rect,
            6.0,
            Stroke::new(1.0, TEXT_HIGHLIGHT),
            StrokeKind::Inside,
        );
        painter.text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            "▶",
            FontId::proportional(14.0),
            TEXT_LIGHT,
        );
    }

    // hover ui
    if response.hovered() {
        painter.rect_filled(rect, 4.0, Color32::from_black_alpha(160));

        let max_width = rect.width() - 12.0;
        let galley = painter.layout(
            item.name.clone(),
            FontId::proportional(13.0),
            Color32::WHITE,
            max_width,
        );

        let galley_size = galley.size();

        let text_pos = rect.center() - galley_size / 2.0;
        painter.galley(text_pos, galley, Color32::WHITE);

        let meta_text = format!("{} / {}", item.category, item.author);
        let meta_galley =
            painter.layout_no_wrap(meta_text, FontId::proportional(11.0), C_TEXT_MUTED);
        let meta_pos = egui::pos2(
            rect.center().x - meta_galley.size().x / 2.0,
            rect.center().y + galley_size.y / 2.0 + 4.0,
        );
        if meta_pos.y + meta_galley.size().y < rect.max.y - 4.0 {
            painter.galley(meta_pos, meta_galley, C_TEXT_MUTED);
        }
    }

    response
}
