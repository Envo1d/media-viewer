use crate::core::models::StagingItem;
use crate::ui::app::MediaApp;
use crate::ui::colors::C_TEXT_MUTED;
use crate::ui::components::grid_view::{
    compute_grid_metrics, BOTTOM_PAD, COL_GAP, ROW_GAP, TOP_PAD,
};
use crate::ui::components::staging_card::staging_card;
use egui::{RichText, Ui, Vec2};
use std::sync::Arc;

pub fn staging_view(app: &mut MediaApp, ui: &mut Ui) {
    if app.staging_items.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);
            ui.label(
                RichText::new("Staging folder is empty")
                    .size(16.0)
                    .color(C_TEXT_MUTED),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Drop media files into your staging folder,\nthen run a staging scan.",
                )
                .size(12.5)
                .color(C_TEXT_MUTED),
            );
        });
        return;
    }

    let card_sz = app.card_size;
    let total_items = app.staging_items.len();
    let m = compute_grid_metrics(ui.available_width(), total_items, card_sz);

    let mut distribute_request: Option<Arc<StagingItem>> = None;
    let mut delete_request: Option<Arc<StagingItem>> = None;

    egui::ScrollArea::vertical()
        .animated(false)
        .wheel_scroll_multiplier(Vec2::splat(2.5))
        .show_rows(ui, m.row_h, m.total_rows, |ui, row_range| {
            if row_range.start == 0 {
                ui.add_space(TOP_PAD);
            }

            for row in row_range {
                ui.horizontal(|ui| {
                    ui.add_space(m.h_pad);

                    for col in 0..m.columns {
                        let idx = row * m.columns + col;
                        let Some(item) = app.staging_items.get(idx) else {
                            break;
                        };

                        staging_card(
                            ui,
                            item,
                            &mut app.texture_manager,
                            card_sz,
                            app.show_previews,
                            &mut distribute_request,
                            &mut delete_request,
                        );

                        if col + 1 < m.columns && idx + 1 < total_items {
                            ui.add_space(COL_GAP);
                        }
                    }

                    ui.add_space(m.h_pad);
                });

                ui.add_space(ROW_GAP);
            }

            ui.add_space(BOTTOM_PAD);
        });

    if let Some(item) = distribute_request {
        app.open_distribute_modal(item);
    }
    if let Some(item) = delete_request {
        app.request_delete_staging(item);
    }
}
