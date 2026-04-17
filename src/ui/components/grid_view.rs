pub const SIDE_PAD: f32 = 18.0;
pub const COL_GAP: f32 = 10.0;
pub const ROW_GAP: f32 = 10.0;
pub const TOP_PAD: f32 = 12.0;
pub const BOTTOM_PAD: f32 = 28.0;

pub struct GridMetrics {
    pub columns: usize,
    pub h_pad: f32,
    pub row_h: f32,
    pub total_rows: usize,
}

pub fn compute_grid_metrics(avail_w: f32, total_items: usize, card_sz: f32) -> GridMetrics {
    let usable_w = (avail_w - SIDE_PAD * 2.0).max(card_sz);
    let columns = ((usable_w + COL_GAP) / (card_sz + COL_GAP))
        .floor()
        .max(1.0) as usize;
    let grid_w = columns as f32 * card_sz + (columns - 1) as f32 * COL_GAP;
    let h_pad = SIDE_PAD + ((usable_w - grid_w) * 0.5).max(0.0);
    let row_h = card_sz + ROW_GAP;
    let total_rows = (total_items + columns - 1) / columns;
    GridMetrics {
        columns,
        h_pad,
        row_h,
        total_rows,
    }
}
