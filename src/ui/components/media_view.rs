use crate::core::models::{MediaItem, PendingDelete, RubberBand};
use crate::ui::app::MediaApp;
use crate::ui::colors::C_BLURPLE;
use crate::ui::components::grid_view::{
    compute_grid_metrics, BOTTOM_PAD, COL_GAP, ROW_GAP, TOP_PAD,
};
use crate::ui::components::media_card::media_card;
use crate::utils::file_helpers::parse_suffix_number;
use egui::scroll_area::ScrollSource;
use egui::{Id, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

const PREFETCH_MARGIN: usize = 2;
const LOAD_AHEAD_PX: f32 = 1200.0;

fn build_group_set(items: &[Arc<MediaItem>]) -> HashSet<usize> {
    let mut key_to_indices: HashMap<String, Vec<usize>> = HashMap::new();

    for (i, item) in items.iter().enumerate() {
        let p = Path::new(&item.path);

        let dir = p
            .parent()
            .map(|d| d.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        let base = if let Some((b, _)) = parse_suffix_number(stem) {
            b.to_lowercase()
        } else {
            stem.to_lowercase()
        };

        key_to_indices
            .entry(format!("{dir}|{base}"))
            .or_default()
            .push(i);
    }

    key_to_indices
        .into_values()
        .filter(|v| v.len() >= 2)
        .flatten()
        .collect()
}

fn draw_rubber_band(ctx: &egui::Context, start: Pos2, end: Pos2) {
    let layer = egui::LayerId::new(egui::Order::Foreground, Id::new("rb_sel"));
    let p = ctx.layer_painter(layer);
    let rect = Rect::from_two_pos(start, end);
    p.rect_filled(rect, 2.0, C_BLURPLE.linear_multiply(0.12));
    p.rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, C_BLURPLE.linear_multiply(0.7)),
        StrokeKind::Outside,
    );
}

pub fn media_view(app: &mut MediaApp, ui: &mut Ui) {
    let card_sz = app.card_size;
    let total_items = app.displayed_items.len();
    let m = compute_grid_metrics(ui.available_width(), total_items, card_sz);
    let group_set = build_group_set(&app.displayed_items);
    let selection_count = app.selection.len();

    let rb_id = Id::new("media_view_rb");
    let rb: RubberBand = ui
        .ctx()
        .memory(|mem| mem.data.get_temp::<RubberBand>(rb_id))
        .unwrap_or_default();

    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.clear_selection();
    }

    let mut edit_request: Option<Arc<MediaItem>> = None;
    let mut delete_request: Option<Arc<MediaItem>> = None;
    let mut reorder_request: Option<Arc<MediaItem>> = None;
    let mut bulk_delete_request: bool = false;
    let mut card_rects: Vec<(usize, Rect)> = Vec::new();
    let mut toggle_paths: Vec<(usize, String)> = Vec::new();

    let out = egui::ScrollArea::vertical()
        .animated(false)
        .scroll_source(ScrollSource::MOUSE_WHEEL)
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

                        let in_group = group_set.contains(&idx);
                        let is_selected = app.selection.contains(&item.path);
                        let mut toggle_this = false;
                        let mut bulk_delete_this = false;

                        let resp = media_card(
                            ui,
                            item,
                            &mut app.texture_manager,
                            card_sz,
                            app.show_previews,
                            in_group,
                            is_selected,
                            selection_count,
                            &mut edit_request,
                            &mut delete_request,
                            &mut reorder_request,
                            &mut bulk_delete_this,
                            &mut toggle_this,
                        );

                        card_rects.push((idx, resp.rect));
                        if toggle_this {
                            toggle_paths.push((idx, item.path.clone()));
                        }
                        if bulk_delete_this {
                            bulk_delete_request = true;
                        }

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

    for (_idx, path) in toggle_paths {
        if app.selection.contains(&path) {
            app.selection.remove(&path);
        } else {
            app.selection.insert(path.clone());
            app.selection_anchor = Some(path);
        }
    }

    let ctx = ui.ctx().clone();
    let inner_rect = out.inner_rect;
    let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
    let primary_pressed = ctx.input(|i| i.pointer.primary_pressed());
    let primary_down = ctx.input(|i| i.pointer.primary_down());
    let primary_released = ctx.input(|i| i.pointer.primary_released());

    let mut new_rb = rb;

    if !new_rb.active && primary_pressed {
        if let Some(pp) = pointer_pos {
            let on_card = card_rects.iter().any(|(_, r)| r.contains(pp));
            if inner_rect.contains(pp) && !on_card {
                new_rb.active = true;
                new_rb.start = pp;
                new_rb.current = pp;
                if !ctx.input(|i| i.modifiers.ctrl) {
                    app.clear_selection();
                }
            }
        }
    }

    if new_rb.active {
        if primary_down {
            if let Some(pp) = pointer_pos {
                new_rb.current = pp;
            }
            draw_rubber_band(&ctx, new_rb.start, new_rb.current);
            ctx.request_repaint();
            let band = Rect::from_two_pos(new_rb.start, new_rb.current).expand(1.0);
            if !ctx.input(|i| i.modifiers.ctrl) {
                app.selection.clear();
            }
            for (idx, card_rect) in &card_rects {
                if band.intersects(*card_rect) {
                    if let Some(item) = app.displayed_items.get(*idx) {
                        app.selection.insert(item.path.clone());
                    }
                }
            }
        } else if primary_released || !primary_down {
            new_rb.active = false;
        }
    }

    ctx.memory_mut(|mem| mem.data.insert_temp(rb_id, new_rb));

    if let Some(item) = edit_request {
        app.clear_selection();
        app.open_edit_modal(item);
    }
    if let Some(item) = delete_request {
        app.pending_delete = Some(PendingDelete::Library(item));
    }
    if let Some(item) = reorder_request {
        app.clear_selection();
        app.open_reorder_modal(item);
    }
    if bulk_delete_request {
        let selected: Vec<Arc<MediaItem>> = app
            .displayed_items
            .iter()
            .filter(|i| app.selection.contains(&i.path))
            .cloned()
            .collect();
        app.pending_delete = Some(PendingDelete::BulkLibrary(selected));
    }

    let scroll_y = out.state.offset.y;
    let content_h = out.content_size.y;
    let visible_h = out.inner_rect.height();

    if content_h > visible_h && scroll_y > content_h - visible_h - LOAD_AHEAD_PX {
        app.load_next_page();
    }
}
