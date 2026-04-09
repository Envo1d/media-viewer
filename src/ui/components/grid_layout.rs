use crate::ui::app::MediaApp;
use crate::ui::components::media_card::media_card;
use egui::{Ui, Vec2};

const SIDE_PAD: f32 = 18.0;
const COL_GAP: f32 = 10.0;
const ROW_GAP: f32 = 10.0;
const PREFETCH_MARGIN: usize = 2;
const LOAD_AHEAD_PX: f32 = 1200.0;
const TOP_PAD: f32 = 12.0;
const BOTTOM_PAD: f32 = 28.0;

pub fn grid_layout(app: &mut MediaApp, ui: &mut Ui) {
    let card_sz = app.card_size;
    let avail_w = ui.available_width();

    let usable_w = (avail_w - SIDE_PAD * 2.0).max(card_sz);

    let columns = ((usable_w + COL_GAP) / (card_sz + COL_GAP))
        .floor()
        .max(1.0) as usize;

    let grid_w = columns as f32 * card_sz + (columns - 1) as f32 * COL_GAP;
    let h_pad = SIDE_PAD + ((usable_w - grid_w) * 0.5).max(0.0);

    let row_h = card_sz + ROW_GAP;
    let total_items = app.displayed_items.len();
    let total_rows = (total_items + columns - 1) / columns;

    let out = egui::ScrollArea::vertical()
        .animated(false)
        .wheel_scroll_multiplier(Vec2::splat(2.5))
        .show_rows(ui, row_h, total_rows, |ui, row_range| {
            if row_range.start == 0 {
                ui.add_space(TOP_PAD);
            }

            let vis_start = row_range.start * columns;
            let vis_end = (row_range.end * columns).min(total_items);

            for row in row_range.clone() {
                ui.horizontal(|ui| {
                    ui.add_space(h_pad);

                    for col in 0..columns {
                        let idx = row * columns + col;
                        let Some(item) = app.displayed_items.get(idx) else {
                            break;
                        };

                        media_card(
                            ui,
                            item,
                            &mut app.texture_manager,
                            card_sz,
                            app.show_previews,
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

            let pre_start = row_range.start.saturating_sub(PREFETCH_MARGIN) * columns;
            let pre_end = (row_range.end + PREFETCH_MARGIN).min(total_rows) * columns;

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

    let scroll_y = out.state.offset.y;
    let content_h = out.content_size.y;
    let visible_h = out.inner_rect.height();

    if content_h > visible_h && scroll_y > content_h - visible_h - LOAD_AHEAD_PX {
        app.load_next_page();
    }
}
