use crate::ui::app::MediaApp;
use crate::ui::components::media_card::media_card;
use egui::{Ui, Vec2};

// ── grid constants ────────────────────────────────────────────────────────────

/// Padding from the left and right panel edges to the first/last card column.
const SIDE_PAD: f32 = 18.0;

/// Gap between columns.
const COL_GAP: f32 = 10.0;

/// Gap between rows. Passed as part of row_height to show_rows so scroll
/// virtualization stays exact (item_spacing.y is 0 in our global style).
const ROW_GAP: f32 = 10.0;

/// Extra rows to prefetch above and below the visible window.
const PREFETCH_MARGIN: usize = 3;

/// Start loading the next page when this many pixels remain until the bottom.
const LOAD_AHEAD_PX: f32 = 1200.0;

const TOP_PAD: f32 = 12.0;
const BOTTOM_PAD: f32 = 28.0;

// ── layout ────────────────────────────────────────────────────────────────────

pub fn grid_layout(app: &mut MediaApp, ui: &mut Ui) {
    let card_sz = app.card_size;
    let avail_w = ui.available_width();

    // The usable interior width is narrower than the panel — cards never touch
    // the side edges.
    let usable_w = (avail_w - SIDE_PAD * 2.0).max(card_sz);

    // How many columns fit inside the usable band?
    let columns = ((usable_w + COL_GAP) / (card_sz + COL_GAP))
        .floor()
        .max(1.0) as usize;

    // Actual grid width; centre it within the usable band.
    let grid_w = columns as f32 * card_sz + (columns - 1) as f32 * COL_GAP;
    let h_pad = SIDE_PAD + ((usable_w - grid_w) * 0.5).max(0.0);

    // Row height fed to show_rows.  Because our global style sets item_spacing.y
    // = 0, show_rows uses this value as-is for virtual-scroll maths.  Adding
    // ROW_GAP here means each row's allocated rect is (card_sz + ROW_GAP) tall;
    // we only draw card_sz of that, so the gap is the empty remainder.
    let row_h = card_sz + ROW_GAP;

    let total_rows = (app.displayed_items.len() + columns - 1) / columns;
    let items = &app.displayed_items;

    let out = egui::ScrollArea::vertical()
        .animated(false)
        .wheel_scroll_multiplier(Vec2::splat(2.5))
        .show_rows(ui, row_h, total_rows, |ui, row_range| {
            // Small top cushion on the very first batch
            if row_range.start == 0 {
                ui.add_space(TOP_PAD);
            }

            // ── prefetch thumbnails for rows just outside the viewport ────────
            let pre_start = row_range.start.saturating_sub(PREFETCH_MARGIN);
            let pre_end = (row_range.end + PREFETCH_MARGIN).min(total_rows);

            for p_row in (pre_start..row_range.start).chain(row_range.end..pre_end) {
                for col in 0..columns {
                    if let Some(item) = items.get(p_row * columns + col) {
                        app.texture_manager.prefetch(&item.path);
                    }
                }
            }

            // ── draw visible rows ─────────────────────────────────────────────
            for row in row_range {
                ui.horizontal(|ui| {
                    // Left padding — keeps cards off the panel edge and centres
                    // the grid within the usable band.
                    ui.add_space(h_pad);

                    for col in 0..columns {
                        let idx = row * columns + col;
                        if idx >= items.len() {
                            break;
                        }
                        if let Some(item) = items.get(idx) {
                            media_card(ui, item, &mut app.texture_manager, card_sz);
                        }
                        // Column gap — only between cards, not after the last one.
                        let next_idx = row * columns + col + 1;
                        if col + 1 < columns && next_idx < items.len() {
                            ui.add_space(COL_GAP);
                        }
                    }

                    ui.add_space(h_pad);
                });

                ui.add_space(COL_GAP);
            }

            ui.add_space(BOTTOM_PAD);
        });

    // ── infinite scroll trigger ───────────────────────────────────────────────
    let scroll_y = out.state.offset.y;
    let content_h = out.content_size.y;
    let visible_h = out.inner_rect.height();

    if content_h > visible_h && scroll_y > content_h - visible_h - LOAD_AHEAD_PX {
        app.load_next_page();
    }
}
