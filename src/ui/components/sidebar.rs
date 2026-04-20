use crate::core::models::{FieldFilter, MediaFilter, SortOrder};
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    C_BLURPLE, C_HOVER, C_INPUT_BG, C_SELECTED, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED,
};
use crate::ui::components::search_input::search_input;
use crate::ui::components::widgets::filter_chip::filter_chip;
use crate::ui::components::widgets::section_heading::section_heading;
use crate::ui::components::widgets::sort_row::sort_row;
use crate::ui::components::widgets::toggle::toggle;
use egui::{Color32, CornerRadius, FontId, Frame, Margin, Pos2, RichText, Sense, Vec2};
use std::time::Instant;

fn stat_chip(ui: &mut egui::Ui, label: &str, count: u32, active: bool) -> bool {
    let desired = Vec2::new(ui.available_width(), 28.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let bg = if active {
            C_BLURPLE
        } else if resp.hovered() {
            C_HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, CornerRadius::same(6), bg);

        if active {
            let stripe = egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
            ui.painter()
                .rect_filled(stripe, CornerRadius::same(2), Color32::WHITE);
        }

        let text_color = if active { C_TEXT_HEADER } else { C_TEXT };
        ui.painter().text(
            Pos2::new(rect.min.x + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.5),
            text_color,
        );

        let badge_color = if active {
            Color32::from_rgba_premultiplied(255, 255, 255, 160)
        } else {
            C_TEXT_MUTED
        };
        ui.painter().text(
            Pos2::new(rect.max.x - 6.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(11.0),
            badge_color,
        );
    }

    resp.clicked()
}

fn tag_flow_chip(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    const H: f32 = 22.0;
    const PX: f32 = 8.0;
    const FONT: f32 = 11.0;

    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), FontId::proportional(FONT), Color32::WHITE)
    });

    let w = galley.rect.width() + PX * 2.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, H), Sense::click());

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let bg = if active {
            C_BLURPLE
        } else if resp.hovered() {
            C_SELECTED
        } else {
            C_INPUT_BG
        };

        ui.painter().rect_filled(rect, CornerRadius::same(4), bg);

        let text_color = if active { C_TEXT_HEADER } else { C_TEXT_MUTED };
        let text_y = rect.center().y - galley.rect.height() / 2.0;
        ui.painter()
            .galley(Pos2::new(rect.min.x + PX, text_y), galley, text_color);
    }

    resp.clicked()
}

pub fn sidebar(app: &mut MediaApp, ui: &mut egui::Ui) {
    let prev_filter = app.filter.clone();
    let prev_sort = app.sort.clone();

    let close_icon = app.icons.as_ref().unwrap().get("close").clone();
    let sr = search_input(
        ui,
        &mut app.search_input,
        "Search...",
        &close_icon,
        "library",
    );
    if sr.changed {
        app.last_input_time = Instant::now();
    }
    if sr.cleared {
        app.last_input_time = Instant::now();
        app.field_filter = None;
        app.refresh_items();
    }

    let count_text = if app.scan_manager.is_scanning {
        format!(
            "Scanning…  {} files indexed",
            app.scan_manager.files_scanned
        )
    } else {
        format!("{} items", app.displayed_items.len())
    };

    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.style_mut().interaction.selectable_labels = false;
        ui.label(RichText::new(count_text).color(C_TEXT_MUTED).size(11.0));
    });

    ui.add_space(6.0);

    section_heading(ui, "FILTER");

    if filter_chip(ui, "All media", matches!(app.filter, MediaFilter::All)) {
        app.filter = MediaFilter::All;
    }
    if filter_chip(ui, "Images", matches!(app.filter, MediaFilter::Images)) {
        app.filter = MediaFilter::Images;
    }
    if filter_chip(ui, "Videos", matches!(app.filter, MediaFilter::Videos)) {
        app.filter = MediaFilter::Videos;
    }

    section_heading(ui, "SORT BY");

    Frame::NONE
        .fill(C_INPUT_BG)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(4))
        .show(ui, |ui| {
            if sort_row(ui, "Name A → Z", matches!(app.sort, SortOrder::NameAsc)) {
                app.sort = SortOrder::NameAsc;
            }
            if sort_row(ui, "Name Z → A", matches!(app.sort, SortOrder::NameDesc)) {
                app.sort = SortOrder::NameDesc;
            }
            if sort_row(ui, "Newest first", matches!(app.sort, SortOrder::DateDesc)) {
                app.sort = SortOrder::DateDesc;
            }
            if sort_row(ui, "Oldest first", matches!(app.sort, SortOrder::DateAsc)) {
                app.sort = SortOrder::DateAsc;
            }
        });

    if !app.sidebar_stats.top_artists.is_empty() {
        section_heading(ui, "TOP ARTISTS");

        let artists: Vec<(String, u32)> = app.sidebar_stats.top_artists.clone();
        let current_ff = app.field_filter.clone();
        for (artist, count) in &artists {
            let active = current_ff
                .as_ref()
                .map(|f| matches!(f, FieldFilter::Artist(v) if v == artist))
                .unwrap_or(false);
            if stat_chip(ui, artist, *count, active) {
                app.toggle_field_filter(FieldFilter::Artist(artist.clone()));
            }
        }
    }

    if !app.sidebar_stats.top_copyrights.is_empty() {
        section_heading(ui, "TOP COPYRIGHTS");

        let copyrights: Vec<(String, u32)> = app.sidebar_stats.top_copyrights.clone();
        let current_ff = app.field_filter.clone();
        for (cr, count) in &copyrights {
            let active = current_ff
                .as_ref()
                .map(|f| matches!(f, FieldFilter::Copyright(v) if v == cr))
                .unwrap_or(false);
            if stat_chip(ui, cr, *count, active) {
                app.toggle_field_filter(FieldFilter::Copyright(cr.clone()));
            }
        }
    }

    if !app.sidebar_stats.top_tags.is_empty() {
        section_heading(ui, "TOP TAGS");

        let tags: Vec<(String, u32)> = app.sidebar_stats.top_tags.clone();
        let current_ff = app.field_filter.clone();

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
            for (tag, _count) in &tags {
                let active = current_ff
                    .as_ref()
                    .map(|f| matches!(f, FieldFilter::Tag(v) if v == tag))
                    .unwrap_or(false);
                if tag_flow_chip(ui, tag, active) {
                    app.toggle_field_filter(FieldFilter::Tag(tag.clone()));
                }
            }
        });
    }

    section_heading(ui, "SHOW PREVIEWS");

    let toggle_id = ui.make_persistent_id("toggle_previews");
    if toggle(ui, toggle_id, &mut app.show_previews) && !app.show_previews {
        app.texture_manager.invalidate_prefetch();
    }

    if app.filter != prev_filter || app.sort != prev_sort {
        app.texture_manager.invalidate_prefetch();
        app.refresh_items();
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(8.0);
        if app.scan_manager.is_scanning {
            ui.add(egui::ProgressBar::new(-1.0).animate(true));
            ui.add_space(6.0);
        }
    });
}
