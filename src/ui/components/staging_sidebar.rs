use crate::ui::app::MediaApp;
use crate::ui::colors::{C_TEXT, C_TEXT_MUTED};
use crate::ui::components::widgets::pill_button::pill_button;
use crate::ui::components::widgets::section_heading::section_heading;
use crate::ui::components::widgets::toggle::toggle;
use crate::utils::icon;
use egui::RichText;

pub fn staging_sidebar(app: &mut MediaApp, ui: &mut egui::Ui) {
    let icons = app.icons.as_ref().unwrap();

    // Title
    ui.add_space(6.0);
    ui.style_mut().interaction.selectable_labels = false;
    ui.horizontal(|ui| {
        icon(ui, icons.get("folder_open"), 15.0);
        ui.add_space(8.0);
        ui.label(
            RichText::new("STAGING INBOX")
                .size(10.5)
                .color(C_TEXT_MUTED),
        );
    });
    ui.add_space(8.0);

    // Item count
    let count = app.staging_items.len();
    ui.label(
        RichText::new(format!(
            "{} file{}",
            count,
            if count == 1 { "" } else { "s" }
        ))
        .size(12.5)
        .color(C_TEXT),
    );

    // Scan progress / Refresh button
    section_heading(ui, "SCAN");

    if app.scan_manager.is_staging_scanning {
        ui.label(
            RichText::new(format!(
                "Scanning… {} indexed",
                app.scan_manager.staging_files_scanned
            ))
            .size(11.0)
            .color(C_TEXT_MUTED),
        );
        ui.add_space(6.0);
        ui.add(egui::ProgressBar::new(-1.0).animate(true));
    } else {
        let has_path = app.config.staging_path.is_some();
        if pill_button(ui, "Refresh scan", has_path) && has_path {
            app.rescan_staging();
        }
        if !has_path {
            ui.add_space(6.0);
            ui.label(
                RichText::new("Configure a staging folder\nin Settings first.")
                    .size(10.5)
                    .color(C_TEXT_MUTED),
            );
        }
    }

    // Preview toggle
    section_heading(ui, "SHOW PREVIEWS");

    let toggle_id = ui.make_persistent_id("toggle_staging_previews");
    if toggle(ui, toggle_id, &mut app.show_previews) && !app.show_previews {
        app.texture_manager.invalidate_prefetch();
    }
}
