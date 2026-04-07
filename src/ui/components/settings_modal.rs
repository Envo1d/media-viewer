use crate::infra::config::AppConfig;
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    BACKDROP, BORDER, CARD_BG, C_BLURPLE, C_INPUT_BG, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED, DANGER
    ,
};
use crate::ui::components::widgets::danger_button::danger_button;
use crate::ui::components::widgets::pill_button::pill_button;
use crate::ui::components::widgets::section_heading::section_heading;
use crate::ui::components::widgets::section_row::section_row;
use crate::ui::components::widgets::toggle::toggle;
use crate::utils::icon;
use egui::{
    Align2, Color32, CornerRadius, Frame, Id, Image, Margin, Pos2, Rect, RichText, Sense, Stroke,
    Vec2,
};
use rfd::FileDialog;

const MODAL_W: f32 = 460.0;

fn dir_size_mb(path: &std::path::Path) -> f64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0.0;
    };
    entries
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum::<u64>() as f64
        / (1024.0 * 1024.0)
}

fn clear_cache(cache_dir: &std::path::Path) {
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn library_section(app: &mut MediaApp, ui: &mut egui::Ui) {
    let icons = app.icons.as_ref().unwrap();
    section_heading(ui, "LIBRARY");

    section_row(ui, true, false, |ui| {
        icon(ui, icons.get("folder"), 16.0);
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new("Library folder").size(12.5).color(C_TEXT));
            let shown = if app.root_path.is_empty() {
                "No folder selected".to_owned()
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
                if let Some(folder) = FileDialog::new().set_directory(start).pick_folder() {
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
}

fn indexing_section(app: &mut MediaApp, ui: &mut egui::Ui) {
    let icons = app.icons.as_ref().unwrap();
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
                "Index all media in the library folder".to_owned()
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
}

fn cache_section(app: &mut MediaApp, ui: &mut egui::Ui) {
    let icons = app.icons.as_ref().unwrap();
    let cache_dir = AppConfig::get_cache_dir();
    let cache_mb = dir_size_mb(&cache_dir) as f32;
    const MAX_MB: f32 = 500.0;
    let frac = (cache_mb / MAX_MB).min(1.0);

    section_heading(ui, "CACHE");

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
                RichText::new(format!("{cache_mb:.1} MB"))
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
}

pub fn settings_modal(app: &mut MediaApp, ui: &egui::Ui) {
    if !app.settings_open.unwrap_or(false) {
        return;
    }

    let ctx = ui.ctx();
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
                                let close_icon = app.icons.as_ref().unwrap().get("close");
                                let icon_rect =
                                    Rect::from_center_size(rect.center(), Vec2::splat(16.0));
                                ui.put(
                                    icon_rect,
                                    Image::new(close_icon)
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
                .fill(CARD_BG)
                .inner_margin(Margin::symmetric(18, 4))
                .show(ui, |ui| {
                    ui.set_width(MODAL_W - 36.0);

                    library_section(app, ui);
                    indexing_section(app, ui);
                    cache_section(app, ui);

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
