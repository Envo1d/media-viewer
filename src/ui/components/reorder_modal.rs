use crate::core::models::{MediaItem, ReorderAction};
use crate::data::db_service::DbService;
use crate::infra::config::AppConfig;
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    BORDER, CARD_BG, C_BLURPLE, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED, DANGER, SECTION_BG,
};
use crate::ui::components::modal_window::{
    modal_backdrop, modal_frame_window, modal_header, modal_separator,
};
use crate::ui::components::widgets::pill_button::pill_button;
use crate::utils::file_helpers::{apply_group_reorder, natural_cmp};
use crossbeam_channel::Receiver;
use egui::{
    Align2, Color32, CursorIcon, FontId, Frame, Id, Margin, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Vec2,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

const MODAL_W: f32 = 500.0;
const MODAL_H: f32 = 640.0;
const INNER_W: f32 = MODAL_W - 40.0;
const ROW_H: f32 = 64.0;
const THUMB_SZ: f32 = 44.0;
const HANDLE_W: f32 = 36.0;
const MAX_LIST_H: f32 = ROW_H * 8.0;
const INSERT_THICK: f32 = 2.5;
const INSERT_CAP_R: f32 = 4.0;

pub struct ReorderState {
    pub items: Vec<Arc<MediaItem>>,

    pub base_stem: String,

    pub ext: String,

    pub dir: PathBuf,

    pub pending_rx: Option<Receiver<Vec<Arc<MediaItem>>>>,

    pub drag_idx: Option<usize>,

    pub insert_slot: usize,

    pub error: Option<String>,
}

impl ReorderState {
    pub fn new(base_stem: String, ext: String, dir: PathBuf) -> Self {
        let dir_str = dir.to_string_lossy().to_string();
        let rx = DbService::query_group(base_stem.clone(), dir_str);
        Self {
            items: Vec::new(),
            base_stem,
            ext,
            dir,
            pending_rx: Some(rx),
            drag_idx: None,
            insert_slot: 0,
            error: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.pending_rx.is_none()
    }

    pub fn poll(&mut self) -> bool {
        let rx = match self.pending_rx.take() {
            Some(r) => r,
            None => return false,
        };
        match rx.try_recv() {
            Ok(mut items) => {
                items.sort_unstable_by(|a, b| natural_cmp(&a.name, &b.name));
                self.items = items;
                true
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {
                self.pending_rx = Some(rx);
                false
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                self.error = Some("DB query failed — try again.".into());
                true
            }
        }
    }
}

fn draw_handle(p: &egui::Painter, cx: f32, cy: f32, color: Color32) {
    let hw = 9.0_f32;
    let gap = 3.5_f32;
    for dy in [-gap, 0.0_f32, gap] {
        p.line_segment(
            [Pos2::new(cx - hw, cy + dy), Pos2::new(cx + hw, cy + dy)],
            Stroke::new(1.5, color),
        );
    }
}

fn draw_insert_line(p: &egui::Painter, y: f32, x0: f32, x1: f32) {
    p.line_segment(
        [Pos2::new(x0, y), Pos2::new(x1, y)],
        Stroke::new(INSERT_THICK, C_BLURPLE),
    );
    p.circle_filled(Pos2::new(x0, y), INSERT_CAP_R, C_BLURPLE);
    p.circle_filled(Pos2::new(x1, y), INSERT_CAP_R, C_BLURPLE);
}

pub fn reorder_modal(app: &mut MediaApp, ui: &egui::Ui) -> ReorderAction {
    if let Some(state) = app.reorder_state.as_mut() {
        if state.poll() {
            ui.ctx().request_repaint();
        }
    }

    let Some(state) = app.reorder_state.as_ref() else {
        return ReorderAction::None;
    };

    let ctx = ui.ctx();
    let mut action = ReorderAction::None;

    if modal_backdrop(ctx, "reorder_backdrop", egui::Order::Middle) {
        action = ReorderAction::Close;
    }

    let items = state.items.clone();
    let is_ready = state.is_ready();
    let drag_idx = state.drag_idx;
    let insert_slot = state.insert_slot;
    let error = state.error.clone();
    let n = items.len();

    let textures: Vec<Option<egui::TextureHandle>> = if is_ready {
        items
            .iter()
            .map(|item| {
                if app.show_previews {
                    Some(app.texture_manager.get(&item.path))
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let close_icon = app.icons.as_ref().unwrap().get("close").clone();

    let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
    let primary_down = ctx.input(|i| i.pointer.primary_down());
    let primary_pressed = ctx.input(|i| i.pointer.primary_pressed());
    let primary_released = ctx.input(|i| i.pointer.primary_released());

    let mut new_drag_idx = drag_idx;
    let mut new_insert_slot = insert_slot;
    let mut commit: Option<(usize /*from*/, usize /*slot*/)> = None;

    let list_h = if is_ready {
        (n as f32 * ROW_H).min(MAX_LIST_H)
    } else {
        80.0
    };

    modal_frame_window("##reorder_modal", MODAL_W, Some(MODAL_H)).show(ctx, |ui| {
        ui.set_width(MODAL_W);

        let subtitle = if is_ready {
            format!("{n} files — drag handles to reorder")
        } else {
            "Loading…".into()
        };

        if modal_header(ui, "Reorder Group", Some(subtitle), 52.0, &close_icon) {
            action = ReorderAction::Close;
        }
        modal_separator(ui);

        if !is_ready {
            Frame::NONE
                .inner_margin(Margin::symmetric(20, 20))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Fetching group from database…")
                                .size(12.0)
                                .color(C_TEXT_MUTED),
                        );
                    });
                });
        } else if n == 0 {
            Frame::NONE
                .inner_margin(Margin::symmetric(20, 20))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("No files found in this group.")
                            .size(12.0)
                            .color(C_TEXT_MUTED),
                    );
                });
        } else {
            let mut handle_rects: Vec<Rect> = Vec::with_capacity(n);
            let mut row_rects: Vec<Rect> = Vec::with_capacity(n);

            Frame::NONE
                .inner_margin(Margin::symmetric(20, 4))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("reorder_scroll")
                        .max_height(list_h)
                        .animated(false)
                        .show(ui, |ui| {
                            ui.set_min_width(INNER_W);

                            for (vi, item) in items.iter().enumerate() {
                                let is_dragged = new_drag_idx == Some(vi);
                                let alpha: f32 = if is_dragged { 0.28 } else { 1.0 };

                                let (row_rect, _) = ui
                                    .allocate_exact_size(Vec2::new(INNER_W, ROW_H), Sense::hover());
                                let handle_rect =
                                    Rect::from_min_size(row_rect.min, Vec2::new(HANDLE_W, ROW_H));

                                row_rects.push(row_rect);
                                handle_rects.push(handle_rect);

                                if !ui.is_rect_visible(row_rect) {
                                    continue;
                                }

                                let p = ui.painter();

                                if vi > 0 {
                                    p.line_segment(
                                        [row_rect.left_top(), row_rect.right_top()],
                                        Stroke::new(1.0, BORDER),
                                    );
                                }

                                let handle_hovered = pointer_pos
                                    .map(|pp| handle_rect.contains(pp))
                                    .unwrap_or(false);
                                let handle_col = if handle_hovered || is_dragged {
                                    C_TEXT
                                } else {
                                    C_TEXT_MUTED
                                };
                                draw_handle(
                                    p,
                                    handle_rect.center().x,
                                    handle_rect.center().y,
                                    handle_col.linear_multiply(alpha),
                                );
                                if handle_hovered && new_drag_idx.is_none() {
                                    ctx.set_cursor_icon(CursorIcon::Grab);
                                }

                                let thumb_x = handle_rect.max.x + 6.0 + THUMB_SZ / 2.0;
                                let thumb_rect = Rect::from_center_size(
                                    Pos2::new(thumb_x, row_rect.center().y),
                                    Vec2::splat(THUMB_SZ),
                                );
                                p.rect_filled(thumb_rect, 4.0, SECTION_BG.linear_multiply(alpha));
                                if let Some(Some(tex)) = textures.get(vi) {
                                    let tsz = tex.size_vec2();
                                    let scale = (THUMB_SZ / tsz.x).min(THUMB_SZ / tsz.y);
                                    let isz = tsz * scale;
                                    let imin = thumb_rect.center() - isz / 2.0;
                                    p.image(
                                        tex.id(),
                                        Rect::from_min_size(imin, isz),
                                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                        Color32::WHITE.linear_multiply(alpha),
                                    );
                                }
                                p.rect_stroke(
                                    thumb_rect,
                                    4.0,
                                    Stroke::new(1.0, BORDER.linear_multiply(alpha)),
                                    StrokeKind::Outside,
                                );

                                let text_x = thumb_rect.max.x + 10.0;
                                let badge_cx = row_rect.max.x - 18.0;
                                let max_w = badge_cx - text_x - 8.0;
                                let font_id = FontId::proportional(12.5);
                                let galley = ui.fonts_mut(|f| {
                                    f.layout_no_wrap(item.name.clone(), font_id.clone(), C_TEXT)
                                });
                                let display_name = if galley.rect.width() > max_w {
                                    let ch = ((max_w / (12.5 * 0.55)) as usize).max(4);
                                    format!(
                                        "{}…",
                                        item.name
                                            .chars()
                                            .take(ch.saturating_sub(1))
                                            .collect::<String>()
                                    )
                                } else {
                                    item.name.clone()
                                };
                                p.text(
                                    Pos2::new(text_x, row_rect.center().y),
                                    Align2::LEFT_CENTER,
                                    &display_name,
                                    FontId::proportional(12.5),
                                    C_TEXT.linear_multiply(alpha),
                                );

                                p.text(
                                    Pos2::new(badge_cx, row_rect.center().y),
                                    Align2::RIGHT_CENTER,
                                    format!("#{}", vi + 1),
                                    FontId::proportional(11.0),
                                    C_TEXT_MUTED.linear_multiply(alpha),
                                );
                            }
                        });
                });

            if new_drag_idx.is_none() && primary_pressed {
                if let Some(pp) = pointer_pos {
                    for (vi, hr) in handle_rects.iter().enumerate() {
                        if hr.contains(pp) {
                            new_drag_idx = Some(vi);
                            new_insert_slot = vi;
                            break;
                        }
                    }
                }
            }

            if let Some(drag_i) = new_drag_idx {
                if primary_down {
                    ctx.set_cursor_icon(CursorIcon::Grabbing);

                    if let Some(pp) = pointer_pos {
                        let mut slot = row_rects.len();
                        for (vi, rr) in row_rects.iter().enumerate() {
                            if pp.y < rr.center().y {
                                slot = vi;
                                break;
                            }
                        }
                        new_insert_slot = slot;
                    }

                    let slot = new_insert_slot;
                    let line_y = if slot == 0 {
                        row_rects.first().map(|r| r.min.y).unwrap_or(0.0)
                    } else if slot >= row_rects.len() {
                        row_rects.last().map(|r| r.max.y).unwrap_or(0.0)
                    } else {
                        let a = &row_rects[slot - 1];
                        let b = &row_rects[slot];
                        (a.max.y + b.min.y) * 0.5
                    };
                    if let Some(first) = row_rects.first() {
                        draw_insert_line(
                            ui.painter(),
                            line_y,
                            first.min.x + HANDLE_W + 2.0,
                            first.max.x - 4.0,
                        );
                    }

                    let ghost_layer =
                        egui::LayerId::new(egui::Order::Foreground, Id::new("reorder_ghost"));
                    let gp = ctx.layer_painter(ghost_layer);
                    if let (Some(pp), Some(item)) = (pointer_pos, items.get(drag_i)) {
                        let gr = Rect::from_min_size(
                            Pos2::new(
                                row_rects.first().map(|r| r.min.x).unwrap_or(pp.x),
                                pp.y - ROW_H * 0.5,
                            ),
                            Vec2::new(INNER_W, ROW_H),
                        );
                        gp.rect_filled(gr, 8.0, CARD_BG);
                        gp.rect_stroke(gr, 8.0, Stroke::new(1.5, C_BLURPLE), StrokeKind::Outside);
                        draw_handle(&gp, gr.min.x + HANDLE_W * 0.5, gr.center().y, C_TEXT);
                        gp.text(
                            Pos2::new(gr.min.x + HANDLE_W + 6.0 + THUMB_SZ + 10.0, gr.center().y),
                            Align2::LEFT_CENTER,
                            &item.name,
                            FontId::proportional(12.5),
                            C_TEXT_HEADER,
                        );
                    }

                    ctx.request_repaint();
                } else if primary_released {
                    commit = Some((drag_i, new_insert_slot));
                    new_drag_idx = None;
                }
            }
        }

        if let Some(ref err) = error {
            Frame::NONE
                .inner_margin(Margin::symmetric(20, 8))
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("⚠  {err}")).size(11.0).color(DANGER),
                        )
                        .wrap(),
                    );
                });
        }

        modal_separator(ui);
        Frame::NONE
            .inner_margin(Margin::symmetric(20, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if pill_button(ui, "Cancel", true) {
                        action = ReorderAction::Close;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok = is_ready && n >= 2;
                        if pill_button(ui, "Apply Order", ok) && ok {
                            action = ReorderAction::Apply;
                        }
                    });
                });
            });
    });

    if let Some(state) = app.reorder_state.as_mut() {
        state.drag_idx = new_drag_idx;
        state.insert_slot = new_insert_slot;

        if let Some((from_i, slot)) = commit {
            let final_slot = if slot > from_i {
                slot.saturating_sub(1)
            } else {
                slot
            }
            .min(state.items.len().saturating_sub(1));

            if from_i != final_slot {
                let item = state.items.remove(from_i);
                state.items.insert(final_slot, item);
                state.error = None;
            }
        }
    }

    action
}

pub fn do_apply_reorder(app: &mut MediaApp) {
    let (new_order, base_stem, ext, dir) = match &app.reorder_state {
        Some(s) => (
            s.items.clone(),
            s.base_stem.clone(),
            s.ext.clone(),
            s.dir.clone(),
        ),
        None => return,
    };

    match apply_group_reorder(&new_order, &base_stem, &ext, &dir) {
        Ok(renames) => {
            DbService::rename_group_batch(renames.clone());

            let rename_map: HashMap<&str, (&str, &str)> = renames
                .iter()
                .map(|(o, _, f, n)| (o.as_str(), (f.as_str(), n.as_str())))
                .collect();

            let modified_map: HashMap<String, i64> = app
                .displayed_items
                .iter()
                .filter(|a| rename_map.contains_key(a.path.as_str()))
                .map(|a| (a.path.clone(), a.modified))
                .collect();

            for arc in &mut app.displayed_items {
                if let Some(&(final_path, final_name)) = rename_map.get(arc.path.as_str()) {
                    *arc = Arc::new(MediaItem {
                        path: final_path.to_owned(),
                        name: final_name.to_owned(),
                        ..(**arc).clone()
                    });
                }
            }
            app.rebuild_display_index();

            let path_pairs: Vec<(String, String)> = renames
                .iter()
                .map(|(o, _, f, _)| (o.clone(), f.clone()))
                .collect();

            let cache_triples: Vec<(String, String, i64)> = renames
                .iter()
                .filter_map(|(orig, _, final_path, _)| {
                    let modified = *modified_map.get(orig)?;
                    Some((orig.clone(), final_path.clone(), modified))
                })
                .collect();

            app.texture_manager.remap_paths(&path_pairs);

            let cache_dir = AppConfig::get_cache_dir();
            std::thread::spawn(move || {
                crate::infra::cache::remap_cache_entries(&cache_dir, &cache_triples);
            });

            app.reorder_state = None;
        }
        Err(e) => {
            if let Some(state) = app.reorder_state.as_mut() {
                state.error = Some(e);
            }
        }
    }
}
