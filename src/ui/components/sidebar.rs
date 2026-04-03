use crate::core::models::{MediaFilter, SortOrder};
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    C_BLURPLE, C_HOVER, C_INPUT_BG, C_SELECTED, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED,
};
use crate::ui::components::search_input::search_input;
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Sense, Vec2};

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(14.0);
    ui.label(RichText::new(text).color(C_TEXT_MUTED).size(11.0));
    ui.add_space(4.0);
}

fn filter_chip(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let desired = Vec2::new(ui.available_width(), 30.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if active {
            C_BLURPLE
        } else if response.hovered() {
            C_HOVER
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);

        let text_color = if active { C_TEXT_HEADER } else { C_TEXT };
        ui.painter().text(
            egui::pos2(rect.min.x + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            text_color,
        );
    }

    response.clicked()
}

fn sort_row(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let desired = Vec2::new(ui.available_width(), 28.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if active {
            C_SELECTED
        } else if response.hovered() {
            C_HOVER
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, CornerRadius::same(5), fill);

        if active {
            let stripe = egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
            ui.painter()
                .rect_filled(stripe, CornerRadius::same(2), C_BLURPLE);
        }

        let text_color = if active { C_TEXT_HEADER } else { C_TEXT };
        ui.painter().text(
            egui::pos2(rect.min.x + 14.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            text_color,
        );
    }

    response.clicked()
}

pub fn sidebar(app: &mut MediaApp, ui: &mut egui::Ui) {
    let prev_filter = app.filter.clone();
    let prev_sort = app.sort.clone();

    // Search input
    search_input(app, ui);

    {
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
            ui.label(RichText::new(count_text).color(C_TEXT_MUTED).size(11.0));
        });
    }

    ui.add_space(6.0);

    section_label(ui, "FILTER");

    if filter_chip(ui, "All media", matches!(app.filter, MediaFilter::All)) {
        app.filter = MediaFilter::All;
    }
    if filter_chip(ui, "Images", matches!(app.filter, MediaFilter::Images)) {
        app.filter = MediaFilter::Images;
    }
    if filter_chip(ui, "Videos", matches!(app.filter, MediaFilter::Videos)) {
        app.filter = MediaFilter::Videos;
    }

    section_label(ui, "SORT BY");

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

    section_label(ui, "CARD SIZE");

    ui.horizontal(|ui| {
        ui.label(RichText::new("S").color(C_TEXT_MUTED).size(11.0));
        ui.add_space(4.0);
        ui.add(
            egui::Slider::new(&mut app.card_size, 120.0..=320.0)
                .show_value(false)
                .step_by(10.0),
        );
        ui.add_space(4.0);
        ui.label(RichText::new("L").color(C_TEXT_MUTED).size(11.0));
    });

    if app.filter != prev_filter || app.sort != prev_sort {
        app.refresh_items();
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(8.0);

        if app.scan_manager.is_scanning {
            ui.add(egui::ProgressBar::new(-1.0).animate(true)); // indeterminate
            ui.add_space(6.0);
        }
    });
}
