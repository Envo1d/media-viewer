use crate::core::models::{FieldFilter, FileDetailInfo, MediaType};
use crate::core::windows_media::query_video_properties;
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    BORDER, C_BLURPLE, C_INPUT_BG, C_PRIMARY_BG, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED,
};
use egui::{
    Color32, CornerRadius, CursorIcon, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Vec2,
};
use image::RgbaImage;

pub const DETAIL_PANEL_W: f32 = 300.0;
const PREVIEW_H: f32 = 220.0;
const INFO_ROW_H: f32 = 28.0;
const LABEL_FONT: f32 = 10.5;
const VALUE_FONT: f32 = 12.0;
const CHIP_H: f32 = 22.0;
const CHIP_PX: f32 = 9.0;
const CHIP_CR: f32 = 4.0;

pub fn load_file_detail(path: &str, media_type: &MediaType) -> FileDetailInfo {
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    match media_type {
        MediaType::Image => {
            let dimensions = image::image_dimensions(path).ok();
            FileDetailInfo {
                file_size,
                dimensions,
                duration_secs: None,
                frame_rate: None,
            }
        }
        MediaType::Video => {
            let vp = query_video_properties(path);
            FileDetailInfo {
                file_size,
                dimensions: match (vp.frame_width, vp.frame_height) {
                    (Some(w), Some(h)) => Some((w, h)),
                    _ => None,
                },
                duration_secs: vp.duration_secs,
                frame_rate: vp.frame_rate,
            }
        }
    }
}

pub fn load_preview_image(path: &str) -> Option<RgbaImage> {
    #[cfg(windows)]
    {
        crate::core::windows_thumb::get_thumbnail(path, 600)
    }
    #[cfg(not(windows))]
    {
        image::open(path).ok().map(|i| i.into_rgba8())
    }
}

fn fmt_bytes(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    const KB: u64 = 1_024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn fmt_timestamp(unix_secs: i64) -> String {
    if unix_secs <= 0 {
        return "—".into();
    }
    let mut days = unix_secs / 86400;
    let sod = unix_secs % 86400;
    let h = sod / 3600;
    let m = (sod % 3600) / 60;
    let mut year = 1970i64;
    loop {
        let dy = if is_leap(year) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }
    let month_lens = [
        31i64,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for &ml in &month_lens {
        if days < ml {
            break;
        }
        days -= ml;
        month += 1;
    }
    format!(
        "{year:04}-{month:02}-{day:02}  {h:02}:{m:02}",
        day = days + 1
    )
}

fn fmt_duration(secs: f64) -> String {
    let total = secs as u64;
    let s = total % 60;
    let m = (total / 60) % 60;
    let h = total / 3600;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn ext_upper(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_uppercase())
        .unwrap_or_else(|| "—".into())
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str, alt_bg: bool) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, INFO_ROW_H), Sense::hover());
    if ui.is_rect_visible(rect) {
        if alt_bg {
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_rgba_premultiplied(0, 0, 0, 20));
        }
        ui.painter().text(
            Pos2::new(rect.min.x + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(LABEL_FONT),
            C_TEXT_MUTED,
        );
        ui.painter().text(
            Pos2::new(rect.max.x - 10.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            value,
            FontId::proportional(VALUE_FONT),
            C_TEXT,
        );
    }
}

fn filter_chip_small(
    ui: &mut egui::Ui,
    label: &str,
    accent: Color32,
    active: bool,
    clickable: bool,
) -> bool {
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), FontId::proportional(11.0), Color32::WHITE)
    });
    let w = galley.rect.width() + CHIP_PX * 2.0;
    let sense = if clickable {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(w, CHIP_H), sense);
    if clickable {
        resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(rect) {
        let bg = if active {
            accent
        } else if clickable && resp.hovered() {
            accent.linear_multiply(0.35)
        } else {
            accent.linear_multiply(0.18)
        };
        ui.painter().rect_filled(rect, CHIP_CR, bg);
        ui.painter().rect_stroke(
            rect,
            CHIP_CR,
            Stroke::new(1.0, accent.linear_multiply(if active { 1.0 } else { 0.45 })),
            StrokeKind::Outside,
        );
        let ty = rect.center().y - galley.rect.height() / 2.0;
        ui.painter()
            .galley(Pos2::new(rect.min.x + CHIP_PX, ty), galley, C_TEXT_HEADER);
    }
    resp.clicked()
}

fn draw_sep(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, BORDER);
}

fn draw_name_row(ui: &mut egui::Ui, name: &str, path: &str, media_type: &MediaType, panel_w: f32) {
    let ext = ext_upper(path);
    let badge_color = match media_type {
        MediaType::Image => Color32::from_rgb(72, 140, 220),
        MediaType::Video => Color32::from_rgb(200, 90, 60),
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
        let bg = ui.fonts_mut(|f| {
            f.layout_no_wrap(ext.clone(), FontId::proportional(9.5), Color32::WHITE)
        });
        let bw = bg.rect.width() + 8.0;
        let (br, _) = ui.allocate_exact_size(Vec2::new(bw, 16.0), Sense::hover());
        if ui.is_rect_visible(br) {
            ui.painter().rect_filled(br, 3.0, badge_color);
            let ty = br.center().y - bg.rect.height() / 2.0;
            ui.painter()
                .galley(Pos2::new(br.min.x + 4.0, ty), bg, Color32::WHITE);
        }
        let max_chars = ((panel_w - bw - 28.0) / 6.5) as usize;
        let display = if name.chars().count() > max_chars && max_chars > 4 {
            format!("{}…", name.chars().take(max_chars - 1).collect::<String>())
        } else {
            name.to_owned()
        };
        ui.add(
            egui::Label::new(
                RichText::new(display)
                    .size(12.5)
                    .color(C_TEXT_HEADER)
                    .strong(),
            )
            .truncate(),
        );
    });
    if name.chars().count() > 30 {
        ui.add_space(2.0);
        ui.add(egui::Label::new(RichText::new(name).size(10.0).color(C_TEXT_MUTED)).wrap());
    }
}

fn draw_preview(
    ui: &mut egui::Ui,
    show_previews: bool,
    texture: Option<&egui::TextureHandle>,
    media_type: &MediaType,
    panel_w: f32,
) {
    let (preview_rect, _) = ui.allocate_exact_size(Vec2::new(panel_w, PREVIEW_H), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(preview_rect, 0.0, C_PRIMARY_BG);

    let show_icon = |painter: &egui::Painter| {
        let icon = if matches!(media_type, MediaType::Image) {
            "🖼"
        } else {
            "🎬"
        };
        painter.text(
            preview_rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            FontId::proportional(36.0),
            Color32::from_gray(90),
        );
    };

    if show_previews {
        if let Some(tex) = texture {
            let tsz = tex.size_vec2();
            let scale = (panel_w / tsz.x).min(PREVIEW_H / tsz.y);
            let isz = tsz * scale;
            let imin = preview_rect.center() - isz / 2.0;
            painter.image(
                tex.id(),
                Rect::from_min_size(imin, isz),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            show_icon(&painter);
        }
    } else {
        show_icon(&painter);
    }

    painter.line_segment(
        [preview_rect.left_bottom(), preview_rect.right_bottom()],
        Stroke::new(1.0, BORDER),
    );
}

fn draw_info_table(
    ui: &mut egui::Ui,
    info: Option<&FileDetailInfo>,
    info_loading: bool,
    panel_w: f32,
    modified: i64,
    path: &str,
    media_type: &MediaType,
) {
    Frame::NONE
        .fill(C_INPUT_BG)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(0))
        .show(ui, |ui| {
            ui.set_min_width(panel_w);
            ui.style_mut().interaction.selectable_labels = false;

            let size_str = if info_loading {
                "Loading…".into()
            } else {
                info.map(|i| fmt_bytes(i.file_size)).unwrap_or("—".into())
            };
            let dim_str = if info_loading {
                ("Loading…".into(), "Loading…".into())
            } else {
                match info.and_then(|i| i.dimensions) {
                    Some((w, h)) => (format!("{w} px"), format!("{h} px")),
                    None => ("—".into(), "—".into()),
                }
            };
            let date_str = fmt_timestamp(modified);
            let type_str = ext_upper(path);

            info_row(ui, "SIZE", &size_str, false);
            info_row(ui, "TYPE", &type_str, true);
            info_row(ui, "MODIFIED", &date_str, false);
            info_row(ui, "WIDTH", &dim_str.0, true);
            info_row(ui, "HEIGHT", &dim_str.1, false);

            if matches!(media_type, MediaType::Video) {
                let dur_str = if info_loading {
                    "Loading…".into()
                } else {
                    info.and_then(|i| i.duration_secs)
                        .map(fmt_duration)
                        .unwrap_or("—".into())
                };
                let fps_str = if info_loading {
                    "Loading…".into()
                } else {
                    info.and_then(|i| i.frame_rate)
                        .map(|f| format!("{f:.3} fps"))
                        .unwrap_or("—".into())
                };
                info_row(ui, "DURATION", &dur_str, true);
                info_row(ui, "FRAME RATE", &fps_str, false);
            }
        });
}

fn draw_open_button(ui: &mut egui::Ui, path: &str) {
    let avail = ui.available_width();
    let (btn_rect, mut resp) = ui.allocate_exact_size(Vec2::new(avail, 34.0), Sense::click());
    resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    if ui.is_rect_visible(btn_rect) {
        let fill = if resp.is_pointer_button_down_on() {
            C_BLURPLE.linear_multiply(0.70)
        } else if resp.hovered() {
            C_BLURPLE.linear_multiply(0.85)
        } else {
            C_BLURPLE
        };
        ui.painter().rect_filled(btn_rect, 6.0, fill);
        ui.painter().text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Open File",
            FontId::proportional(12.5),
            Color32::WHITE,
        );
    }
    if resp.clicked() {
        let _ = open::that(path);
    }
}

fn draw_distribute_button(ui: &mut egui::Ui) -> bool {
    let avail = ui.available_width();
    let (btn_rect, mut resp) = ui.allocate_exact_size(Vec2::new(avail, 34.0), Sense::click());
    resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    if ui.is_rect_visible(btn_rect) {
        let accent = Color32::from_rgb(60, 160, 100);
        let fill = if resp.is_pointer_button_down_on() {
            accent.linear_multiply(0.70)
        } else if resp.hovered() {
            accent.linear_multiply(0.85)
        } else {
            accent
        };
        ui.painter().rect_filled(btn_rect, 6.0, fill);
        ui.painter().text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Distribute…",
            FontId::proportional(12.5),
            Color32::WHITE,
        );
    }
    resp.clicked()
}

fn draw_empty(ui: &mut egui::Ui, hint: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(140.0);
        ui.label(
            RichText::new("No file selected")
                .size(13.0)
                .color(C_TEXT_MUTED),
        );
        ui.add_space(6.0);
        ui.label(RichText::new(hint).size(11.0).color(C_TEXT_MUTED));
    });
}

pub fn detail_panel(app: &mut MediaApp, ui: &mut egui::Ui) {
    use crate::core::models::ViewMode;
    match app.view_mode {
        ViewMode::Library => detail_panel_library(app, ui),
        ViewMode::Staging => detail_panel_staging(app, ui),
    }
}

fn detail_panel_library(app: &mut MediaApp, ui: &mut egui::Ui) {
    let panel_w = ui.available_width();

    let item = match app.library_detail.selected_item.clone() {
        Some(i) => i,
        None => {
            draw_empty(ui, "Click a card to preview");
            return;
        }
    };

    let tex_ref = app.library_detail.preview_texture.as_ref();
    draw_preview(ui, app.show_previews, tex_ref, &item.media_type, panel_w);

    egui::ScrollArea::vertical()
        .id_salt("detail_lib_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.set_min_width(panel_w);
            ui.add_space(10.0);

            Frame::NONE
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    draw_name_row(ui, &item.name, &item.path, &item.media_type, panel_w);
                });
            ui.add_space(10.0);
            draw_sep(ui);
            ui.add_space(8.0);

            if !item.copyright.is_empty() {
                Frame::NONE
                    .inner_margin(Margin::symmetric(10, 0))
                    .show(ui, |ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(
                            RichText::new("COPYRIGHT")
                                .size(LABEL_FONT)
                                .color(C_TEXT_MUTED),
                        );
                        ui.add_space(4.0);
                        let active = app
                            .field_filter
                            .as_ref()
                            .map(|f| matches!(f, FieldFilter::Copyright(v) if v == &item.copyright))
                            .unwrap_or(false);
                        if filter_chip_small(
                            ui,
                            &item.copyright,
                            Color32::from_rgb(200, 60, 80),
                            active,
                            true,
                        ) {
                            app.toggle_field_filter(FieldFilter::Copyright(item.copyright.clone()));
                        }
                    });
                ui.add_space(8.0);
            }

            if !item.artist.is_empty() {
                Frame::NONE
                    .inner_margin(Margin::symmetric(10, 0))
                    .show(ui, |ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(RichText::new("ARTIST").size(LABEL_FONT).color(C_TEXT_MUTED));
                        ui.add_space(4.0);
                        let active = app
                            .field_filter
                            .as_ref()
                            .map(|f| matches!(f, FieldFilter::Artist(v) if v == &item.artist))
                            .unwrap_or(false);
                        if filter_chip_small(ui, &item.artist, C_BLURPLE, active, true) {
                            app.toggle_field_filter(FieldFilter::Artist(item.artist.clone()));
                        }
                    });
                ui.add_space(8.0);
            }

            if !item.characters.is_empty() {
                Frame::NONE
                    .inner_margin(Margin::symmetric(10, 0))
                    .show(ui, |ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(
                            RichText::new("CHARACTERS")
                                .size(LABEL_FONT)
                                .color(C_TEXT_MUTED),
                        );
                        ui.add_space(4.0);
                        let chars = item.characters.clone();
                        let mut toggle_ch: Option<String> = None;
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                            for ch in &chars {
                                let active = app.active_characters.contains(ch.as_str());
                                if filter_chip_small(
                                    ui,
                                    ch,
                                    Color32::from_rgb(60, 160, 120),
                                    active,
                                    true,
                                ) {
                                    toggle_ch = Some(ch.clone());
                                }
                            }
                        });
                        if let Some(c) = toggle_ch {
                            app.toggle_character(c);
                        }
                    });
                ui.add_space(8.0);
            }

            if !item.tags.is_empty() {
                Frame::NONE
                    .inner_margin(Margin::symmetric(10, 0))
                    .show(ui, |ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(RichText::new("TAGS").size(LABEL_FONT).color(C_TEXT_MUTED));
                        ui.add_space(4.0);
                        let tags = item.tags.clone();
                        let mut toggle_tag: Option<String> = None;
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                            for tag in &tags {
                                let active = app.active_tags.contains(tag.as_str());
                                if filter_chip_small(
                                    ui,
                                    tag,
                                    Color32::from_rgb(130, 90, 210),
                                    active,
                                    true,
                                ) {
                                    toggle_tag = Some(tag.clone());
                                }
                            }
                        });
                        if let Some(t) = toggle_tag {
                            app.toggle_tag(t);
                        }
                    });
                ui.add_space(8.0);
            }

            draw_sep(ui);
            ui.add_space(4.0);

            let info_ref = app.library_detail.info.as_ref();
            let info_loading = info_ref.is_none() && app.library_detail.info_rx.is_some();
            Frame::NONE
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    draw_info_table(
                        ui,
                        info_ref,
                        info_loading,
                        panel_w - 20.0,
                        item.modified,
                        &item.path,
                        &item.media_type,
                    );
                });

            ui.add_space(12.0);
            draw_sep(ui);
            ui.add_space(10.0);

            Frame::NONE
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    draw_open_button(ui, &item.path);
                });

            ui.add_space(12.0);
        });
}

fn detail_panel_staging(app: &mut MediaApp, ui: &mut egui::Ui) {
    let panel_w = ui.available_width();

    let item = match app.staging_detail.selected_item.clone() {
        Some(i) => i,
        None => {
            draw_empty(ui, "Click a card to preview");
            return;
        }
    };

    let tex_ref = app.staging_detail.preview_texture.as_ref();
    draw_preview(ui, app.show_previews, tex_ref, &item.media_type, panel_w);

    let mut distribute_clicked = false;

    egui::ScrollArea::vertical()
        .id_salt("detail_stg_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.set_min_width(panel_w);
            ui.add_space(10.0);

            Frame::NONE
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    draw_name_row(ui, &item.name, &item.path, &item.media_type, panel_w);
                });
            ui.add_space(10.0);
            draw_sep(ui);
            ui.add_space(8.0);

            Frame::NONE
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    ui.style_mut().interaction.selectable_labels = false;
                    ui.label(
                        RichText::new("Staging file — no metadata assigned yet")
                            .size(10.5)
                            .color(C_TEXT_MUTED),
                    );
                });
            ui.add_space(8.0);

            draw_sep(ui);
            ui.add_space(4.0);

            let info_ref = app.staging_detail.info.as_ref();
            let info_loading = info_ref.is_none() && app.staging_detail.info_rx.is_some();
            Frame::NONE
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    draw_info_table(
                        ui,
                        info_ref,
                        info_loading,
                        panel_w - 20.0,
                        item.modified,
                        &item.path,
                        &item.media_type,
                    );
                });

            ui.add_space(12.0);
            draw_sep(ui);
            ui.add_space(10.0);

            Frame::NONE
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    if draw_distribute_button(ui) {
                        distribute_clicked = true;
                    }
                    ui.add_space(6.0);
                    draw_open_button(ui, &item.path);
                });

            ui.add_space(12.0);
        });

    if distribute_clicked {
        app.open_distribute_modal(item);
    }
}
