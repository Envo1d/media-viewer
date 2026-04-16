use crate::core::models::StagingItem;
use crate::ui::app::MediaApp;
use crate::ui::colors::C_TEXT_MUTED;
use crate::ui::components::staging_card::staging_card;
use egui::{RichText, Ui, Vec2};
use std::sync::Arc;

const SIDE_PAD: f32 = 18.0;
const COL_GAP: f32 = 10.0;
const ROW_GAP: f32 = 10.0;
const TOP_PAD: f32 = 12.0;
const BOTTOM_PAD: f32 = 28.0;

pub fn staging_view(app: &mut MediaApp, ui: &mut Ui) {
    let card_sz = app.card_size;
    let avail_w = ui.available_width();

    // Empty state
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

    let usable_w = (avail_w - SIDE_PAD * 2.0).max(card_sz);
    let columns = ((usable_w + COL_GAP) / (card_sz + COL_GAP))
        .floor()
        .max(1.0) as usize;
    let grid_w = columns as f32 * card_sz + (columns - 1) as f32 * COL_GAP;
    let h_pad = SIDE_PAD + ((usable_w - grid_w) * 0.5).max(0.0);
    let row_h = card_sz + ROW_GAP;

    let total_items = app.staging_items.len();
    let total_rows = (total_items + columns - 1) / columns;

    let mut distribute_request: Option<Arc<StagingItem>> = None;
    let mut delete_request: Option<Arc<StagingItem>> = None;

    egui::ScrollArea::vertical()
        .animated(false)
        .wheel_scroll_multiplier(Vec2::splat(2.5))
        .show_rows(ui, row_h, total_rows, |ui, row_range| {
            if row_range.start == 0 {
                ui.add_space(TOP_PAD);
            }

            for row in row_range {
                ui.horizontal(|ui| {
                    ui.add_space(h_pad);
                    for col in 0..columns {
                        let idx = row * columns + col;
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

                        if col + 1 < columns && idx + 1 < total_items {
                            ui.add_space(COL_GAP);
                        }
                    }
                    ui.add_space(h_pad);
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
