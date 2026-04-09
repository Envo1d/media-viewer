use crate::core::models::{MediaFilter, SortOrder};
use crate::ui::app::MediaApp;
use crate::ui::colors::{C_INPUT_BG, C_TEXT_MUTED};
use crate::ui::components::search_input::search_input;
use crate::ui::components::widgets::filter_chip::filter_chip;
use crate::ui::components::widgets::section_heading::section_heading;
use crate::ui::components::widgets::sort_row::sort_row;
use egui::{CornerRadius, Frame, Margin, RichText};

pub fn sidebar(app: &mut MediaApp, ui: &mut egui::Ui) {
    let prev_filter = app.filter.clone();
    let prev_sort = app.sort.clone();

    search_input(app, ui);

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

    section_heading(ui, "CARD SIZE");

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
