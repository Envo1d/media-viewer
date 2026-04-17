use crate::core::models::MediaItem;
use crate::ui::app::MediaApp;
use crate::ui::components::grid_view::{
    compute_grid_metrics, BOTTOM_PAD, COL_GAP, ROW_GAP, TOP_PAD,
};
use crate::ui::components::media_card::media_card;
use egui::{Ui, Vec2};
use std::sync::Arc;

const PREFETCH_MARGIN: usize = 2;
const LOAD_AHEAD_PX: f32 = 1200.0;

pub fn media_view(app: &mut MediaApp, ui: &mut Ui) {
    let card_sz = app.card_size;
    let total_items = app.displayed_items.len();
    let m = compute_grid_metrics(ui.available_width(), total_items, card_sz);

    let mut edit_request: Option<Arc<MediaItem>> = None;
    let mut delete_request: Option<Arc<MediaItem>> = None;

    let out = egui::ScrollArea::vertical()
        .animated(false)
        .wheel_scroll_multiplier(Vec2::splat(2.5))
        .show_rows(ui, m.row_h, m.total_rows, |ui, row_range| {
            if row_range.start == 0 {
                ui.add_space(TOP_PAD);
            }

            let vis_start = row_range.start * m.columns;
            let vis_end = (row_range.end * m.columns).min(total_items);

            for row in row_range.clone() {
                ui.horizontal(|ui| {
                    ui.add_space(m.h_pad);

                    for col in 0..m.columns {
                        let idx = row * m.columns + col;
                        let Some(item) = app.displayed_items.get(idx) else {
                            break;
                        };

                        media_card(
                            ui,
                            item,
                            &mut app.texture_manager,
                            card_sz,
                            app.show_previews,
                            &mut edit_request,
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

            let pre_start = row_range.start.saturating_sub(PREFETCH_MARGIN) * m.columns;
            let pre_end = (row_range.end + PREFETCH_MARGIN).min(m.total_rows) * m.columns;

            for idx in pre_start..vis_start {
                if let Some(item) = app.displayed_items.get(idx) {
                    app.texture_manager.prefetch(&item.path);
                }
            }
            for idx in vis_end..pre_end.min(total_items) {
                if let Some(item) = app.displayed_items.get(idx) {
                    app.texture_manager.prefetch(&item.path);
                }
            }
        });

    if let Some(item) = edit_request {
        app.open_edit_modal(item);
    }
    if let Some(item) = delete_request {
        app.request_delete_library(item);
    }

    let scroll_y = out.state.offset.y;
    let content_h = out.content_size.y;
    let visible_h = out.inner_rect.height();

    if content_h > visible_h && scroll_y > content_h - visible_h - LOAD_AHEAD_PX {
        app.load_next_page();
    }
}
