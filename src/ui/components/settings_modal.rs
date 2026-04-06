use crate::infra::config::AppConfig;
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    BACKDROP, BORDER, CARD_BG, C_BLURPLE, C_INPUT_BG, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED, DANGER,
    DANGER_HOVER, SECTION_BG,
};
use crate::utils::icon;
use egui::{
    Align2, Color32, CornerRadius, FontId, Frame, Id, Image, Margin, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, Vec2,
};
use rfd::FileDialog;

const MODAL_W: f32 = 460.0;
const ROW_H: f32 = 54.0;
const SECTION_CR: u8 = 10;

fn dir_size_mb(path: &std::path::Path) -> f64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0.0;
    };
    let bytes: u64 = entries
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum();
    bytes as f64 / (1024.0 * 1024.0)
}

fn clear_cache(cache_dir: &std::path::Path) {
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn toggle(ui: &mut egui::Ui, id: Id, value: &mut bool) -> bool {
    const W: f32 = 38.0;
    const H: f32 = 22.0;

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(W, H), Sense::click());
    if resp.clicked() {
        *value = !*value;
    }

    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool(id, *value);
        let track = Color32::from_rgb(
            lerp_u8(C_INPUT_BG.r(), C_BLURPLE.r(), t),
            lerp_u8(C_INPUT_BG.g(), C_BLURPLE.g(), t),
            lerp_u8(C_INPUT_BG.b(), C_BLURPLE.b(), t),
        );
        let p = ui.painter();
        p.rect_filled(rect, H / 2.0, track);
        p.rect_stroke(rect, H / 2.0, Stroke::new(1.0, BORDER), StrokeKind::Outside);
        let knob_x = rect.min.x + H / 2.0 + t * (W - H);
        p.circle_filled(
            Pos2::new(knob_x, rect.center().y),
            H / 2.0 - 3.0,
            Color32::WHITE,
        );
    }

    resp.clicked()
}

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

fn section_heading(ui: &mut egui::Ui, label: &str) {
    ui.add_space(16.0);
    ui.label(RichText::new(label).size(10.5).color(C_TEXT_MUTED));
    ui.add_space(4.0);
}

fn section_row(
    ui: &mut egui::Ui,
    is_first: bool,
    is_last: bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    let cr = CornerRadius {
        nw: if is_first { SECTION_CR } else { 0 },
        ne: if is_first { SECTION_CR } else { 0 },
        sw: if is_last { SECTION_CR } else { 0 },
        se: if is_last { SECTION_CR } else { 0 },
    };
    Frame::NONE
        .fill(SECTION_BG)
        .corner_radius(cr)
        .inner_margin(Margin::symmetric(14, 0))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(ui.available_width(), ROW_H));
            ui.horizontal(|ui| {
                ui.set_min_height(ROW_H);
                content(ui);
            });
        });
    if !is_last {
        let (sep, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(sep, 0.0, BORDER);
    }
}

fn pill_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    let px = 14.0;
    let py = 5.0;
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_string(), FontId::proportional(12.0), C_TEXT_HEADER)
    });
    let sz = Vec2::new(
        galley.rect.width() + px * 2.0,
        galley.rect.height() + py * 2.0,
    );
    let (rect, resp) = ui.allocate_exact_size(
        sz,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if ui.is_rect_visible(rect) {
        let fill = if !enabled {
            Color32::from_rgba_premultiplied(255, 255, 255, 6)
        } else if resp.is_pointer_button_down_on() {
            C_BLURPLE.linear_multiply(0.7)
        } else if resp.hovered() {
            C_BLURPLE.linear_multiply(0.85)
        } else {
            C_BLURPLE
        };
        let text_color = if enabled { C_TEXT_HEADER } else { C_TEXT_MUTED };
        ui.painter().rect_filled(rect, 6.0, fill);
        ui.painter()
            .galley(rect.min + Vec2::new(px, py), galley, text_color);
    }
    resp.clicked() && enabled
}

fn danger_button(ui: &mut egui::Ui, label: &str) -> bool {
    let px = 14.0;
    let py = 5.0;
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            label.to_string(),
            FontId::proportional(12.0),
            Color32::WHITE,
        )
    });
    let sz = Vec2::new(
        galley.rect.width() + px * 2.0,
        galley.rect.height() + py * 2.0,
    );
    let (rect, resp) = ui.allocate_exact_size(sz, Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if resp.is_pointer_button_down_on() {
            DANGER.linear_multiply(0.7)
        } else if resp.hovered() {
            DANGER_HOVER
        } else {
            DANGER
        };
        ui.painter().rect_filled(rect, 6.0, fill);
        ui.painter()
            .galley(rect.min + Vec2::new(px, py), galley, Color32::WHITE);
    }
    resp.clicked()
}

pub fn settings_modal(app: &mut MediaApp, ui: &egui::Ui) {
    let ctx = ui.ctx();
    if !app.settings_open.unwrap_or(false) {
        return;
    }
    let icons = app.icons.as_ref().unwrap();

    let screen = ctx.content_rect();
    let mut close = false;

    egui::Area::new(Id::new("settings_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            let resp = ui.allocate_rect(screen, Sense::click());
            ui.painter().rect_filled(screen, 0.0, BACKDROP);
            if resp.clicked() {
                close = true;
            }
        });

    egui::Window::new("##settings_modal")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_size([MODAL_W, 0.0])
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            Frame::NONE
                .fill(CARD_BG)
                .corner_radius(CornerRadius::same(14))
                .stroke(Stroke::new(1.0, BORDER))
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 40,
                    spread: 0,
                    color: Color32::from_black_alpha(120),
                }),
        )
        .show(ctx, |ui| {
            ui.set_width(MODAL_W);

            Frame::NONE
                .inner_margin(Margin::symmetric(20, 0))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(MODAL_W - 40.0, 56.0));
                    ui.horizontal(|ui| {
                        ui.set_min_height(56.0);
                        ui.label(
                            RichText::new("Settings")
                                .size(16.0)
                                .color(C_TEXT_HEADER)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (rect, resp) =
                                ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                            if ui.is_rect_visible(rect) {
                                if resp.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        7.0,
                                        Color32::from_rgba_premultiplied(255, 255, 255, 12),
                                    );
                                }
                                let icon_rect =
                                    Rect::from_center_size(rect.center(), Vec2::splat(16.0));
                                ui.put(
                                    icon_rect,
                                    Image::new(icons.get("close"))
                                        .fit_to_exact_size(Vec2::splat(16.0))
                                        .tint(C_TEXT_MUTED),
                                );
                            }
                            if resp.clicked() {
                                close = true;
                            }
                        });
                    });
                });

            let (sep, _) = ui.allocate_exact_size(Vec2::new(MODAL_W, 1.0), Sense::hover());
            ui.painter().rect_filled(sep, 0.0, BORDER);

            Frame::NONE
                .inner_margin(Margin::symmetric(18, 4))
                .show(ui, |ui| {
                    ui.set_width(MODAL_W - 36.0);

                    section_heading(ui, "LIBRARY");

                    section_row(ui, true, false, |ui| {
                        icon(ui, icons.get("folder"), 16.0);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new("Library folder").size(12.5).color(C_TEXT));
                            let shown = if app.root_path.is_empty() {
                                "No folder selected".to_string()
                            } else {
                                let p = &app.root_path;
                                if p.len() > 42 {
                                    format!("…{}", &p[p.len() - 40..])
                                } else {
                                    p.clone()
                                }
                            };
                            ui.label(RichText::new(shown).size(10.5).color(C_TEXT_MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if pill_button(ui, "Browse", true) {
                                let start = if app.root_path.is_empty() {
                                    std::path::PathBuf::from("/")
                                } else {
                                    std::path::PathBuf::from(&app.root_path)
                                };
                                if let Some(folder) =
                                    FileDialog::new().set_directory(start).pick_folder()
                                {
                                    app.root_path = folder.to_string_lossy().to_string();
                                    app.config.library_path = Some(folder.into());
                                    let _ = app.config.save();
                                }
                            }
                        });
                    });

                    section_row(ui, false, true, |ui| {
                        icon(ui, icons.get("folder_open"), 16.0);
                        ui.add_space(10.0);
                        ui.label(RichText::new("Open in Explorer").size(12.5).color(C_TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let en = !app.root_path.is_empty();
                            if pill_button(ui, "Open", en) && en {
                                let _ = std::process::Command::new("explorer")
                                    .arg(&app.root_path)
                                    .spawn();
                            }
                        });
                    });

                    section_heading(ui, "INDEXING");

                    section_row(ui, true, false, |ui| {
                        icon(ui, icons.get("search"), 16.0);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new("Scan library").size(12.5).color(C_TEXT));
                            let sub = if app.scan_manager.is_scanning {
                                format!("{} files indexed…", app.scan_manager.files_scanned)
                            } else {
                                "Index all media in the library folder".into()
                            };
                            ui.label(RichText::new(sub).size(10.5).color(C_TEXT_MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if app.scan_manager.is_scanning {
                                ui.spinner();
                            } else {
                                let en = !app.root_path.is_empty();
                                if pill_button(ui, "Scan now", en) && en {
                                    app.scan_manager.start(app.root_path.clone());
                                }
                            }
                        });
                    });

                    section_row(ui, false, true, |ui| {
                        icon(ui, icons.get("lightning"), 16.0);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new("Auto-scan on startup")
                                    .size(12.5)
                                    .color(C_TEXT),
                            );
                            ui.label(
                                RichText::new("Scan the library every time Nexa opens")
                                    .size(10.5)
                                    .color(C_TEXT_MUTED),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut val = app.config.auto_scan;
                            if toggle(ui, Id::new("toggle_auto_scan"), &mut val) {
                                app.config.auto_scan = val;
                                let _ = app.config.save();
                            }
                        });
                    });

                    section_heading(ui, "CACHE");

                    let cache_dir = AppConfig::get_cache_dir();
                    let cache_mb = dir_size_mb(&cache_dir) as f32;
                    const MAX_MB: f32 = 500.0;
                    let frac = (cache_mb / MAX_MB).min(1.0);

                    section_row(ui, true, false, |ui| {
                        icon(ui, icons.get("layers"), 16.0);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new("Thumbnail cache").size(12.5).color(C_TEXT));
                            let loc = cache_dir.to_string_lossy();
                            let short = if loc.len() > 40 {
                                format!("…{}", &loc[loc.len() - 38..])
                            } else {
                                loc.to_string()
                            };
                            ui.label(RichText::new(short).size(10.5).color(C_TEXT_MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{:.1} MB", cache_mb))
                                    .size(12.0)
                                    .color(C_TEXT_MUTED),
                            );
                        });
                    });

                    section_row(ui, false, false, |ui| {
                        let w = ui.available_width();
                        let (bar, _) = ui.allocate_exact_size(Vec2::new(w, 6.0), Sense::hover());
                        let p = ui.painter();
                        p.rect_filled(bar, 3.0, C_INPUT_BG);
                        if frac > 0.0 {
                            let fill_color = if frac > 0.9 {
                                DANGER
                            } else if frac > 0.7 {
                                Color32::from_rgb(220, 150, 40)
                            } else {
                                C_BLURPLE
                            };
                            p.rect_filled(
                                Rect::from_min_size(bar.min, Vec2::new(bar.width() * frac, 6.0)),
                                3.0,
                                fill_color,
                            );
                        }
                    });

                    section_row(ui, false, true, |ui| {
                        icon(ui, icons.get("trash"), 16.0);
                        ui.add_space(10.0);
                        ui.label(RichText::new("Clear cache").size(12.5).color(C_TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if danger_button(ui, "Clear") {
                                clear_cache(&cache_dir);
                            }
                        });
                    });

                    ui.add_space(18.0);

                    let (fsep, _) =
                        ui.allocate_exact_size(Vec2::new(MODAL_W - 36.0, 1.0), Sense::hover());
                    ui.painter().rect_filled(fsep, 0.0, BORDER);
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Nexa").size(11.0).color(C_TEXT_MUTED));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                .size(11.0)
                                .color(C_TEXT_MUTED),
                        );
                    });

                    ui.add_space(12.0);
                });
        });

    if close {
        app.settings_open = None;
    }
}
