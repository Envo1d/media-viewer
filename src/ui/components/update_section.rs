use crate::core::models::UpdateState;
use crate::infra::updater::current_version;
use crate::ui::app::MediaApp;
use crate::ui::colors::{BORDER, C_BLURPLE, C_INPUT_BG, C_TEXT, C_TEXT_MUTED, DANGER, SECTION_BG};
use crate::ui::components::widgets::danger_button::danger_button;
use crate::ui::components::widgets::pill_button::pill_button;
use crate::ui::components::widgets::section_heading::section_heading;
use crate::ui::components::widgets::section_row::section_row;
use crate::ui::components::widgets::toggle::toggle;
use egui::{CornerRadius, Frame, Id, Margin, RichText, Sense, Vec2};

fn bytes_human(b: u64) -> String {
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;
    if b >= MB {
        format!("{:.1} MB", b as f64 / MB as f64)
    } else if b >= KB {
        format!("{:.0} KB", b as f64 / KB as f64)
    } else {
        format!("{b} B")
    }
}

fn draw_downloading(
    app: &mut MediaApp,
    ui: &mut egui::Ui,
    version: &str,
    progress: f32,
    bytes_done: u64,
    total_bytes: u64,
) {
    Frame::NONE
        .fill(SECTION_BG)
        .corner_radius(CornerRadius::same(0))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            // Title row + cancel button
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Downloading v{version}…"))
                        .size(12.5)
                        .color(C_TEXT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if danger_button(ui, "Cancel") {
                        app.cancel_update_download();
                    }
                });
            });

            ui.add_space(8.0);

            let bar_w = ui.available_width();
            let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 6.0), Sense::hover());
            ui.painter().rect_filled(bar_rect, 3.0, C_INPUT_BG);
            if progress > 0.0 {
                let fill = egui::Rect::from_min_size(
                    bar_rect.min,
                    Vec2::new((bar_rect.width() * progress).max(0.0), 6.0),
                );
                ui.painter().rect_filled(fill, 3.0, C_BLURPLE);
            }

            ui.add_space(5.0);

            let pct = (progress * 100.0) as u32;
            let detail = if total_bytes > 0 {
                format!(
                    "{}%  —  {} / {}",
                    pct,
                    bytes_human(bytes_done),
                    bytes_human(total_bytes)
                )
            } else {
                format!("{} downloaded", bytes_human(bytes_done))
            };
            ui.label(RichText::new(detail).size(10.5).color(C_TEXT_MUTED));
        });

    let (sep, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(sep, 0.0, BORDER);
}

pub fn update_section(app: &mut MediaApp, ui: &mut egui::Ui) {
    section_heading(ui, "UPDATES");

    section_row(ui, true, false, |ui| {
        ui.vertical(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new("Current version").size(12.5).color(C_TEXT));
            ui.label(
                RichText::new(format!("v{}", current_version()))
                    .size(10.5)
                    .color(C_TEXT_MUTED),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let checking = matches!(app.update_state, UpdateState::Checking);
            let can_check = !matches!(
                app.update_state,
                UpdateState::Downloading { .. } | UpdateState::ReadyToInstall { .. }
            );
            if checking {
                ui.spinner();
            } else if pill_button(ui, "Check now", can_check) && can_check {
                app.start_update_check();
            }
        });
    });

    match app.update_state.clone() {
        UpdateState::UpToDate => {
            section_row(ui, false, false, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("You're up to date").size(12.5).color(C_TEXT));
                    ui.label(
                        RichText::new("No newer version found on GitHub Releases")
                            .size(10.5)
                            .color(C_TEXT_MUTED),
                    );
                });
            });
        }

        UpdateState::Available {
            version,
            size_bytes,
            ..
        } => {
            section_row(ui, false, false, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!("v{version} is available"))
                            .size(12.5)
                            .color(C_TEXT),
                    );
                    ui.label(
                        RichText::new(format!("Download size: {}", bytes_human(size_bytes)))
                            .size(10.5)
                            .color(C_TEXT_MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, "Download", true) {
                        app.start_update_download();
                    }
                });
            });
        }

        UpdateState::Downloading {
            version,
            progress,
            bytes_done,
            total_bytes,
        } => {
            draw_downloading(app, ui, &version, progress, bytes_done, total_bytes);
        }

        UpdateState::ReadyToInstall { version, .. } => {
            section_row(ui, false, false, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!("v{version} ready to install"))
                            .size(12.5)
                            .color(C_TEXT),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new("Nexa will close, update, and restart automatically.")
                                .size(10.5)
                                .color(C_TEXT_MUTED),
                        )
                        .wrap(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, "Restart & Install", true) {
                        app.apply_update();
                    }
                });
            });
        }

        UpdateState::Error(msg) => {
            section_row(ui, false, false, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("⚠  {msg}")).size(11.5).color(DANGER),
                        )
                        .wrap(),
                    );
                    ui.add_space(10.0);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, "Retry", true) {
                        app.start_update_check();
                    }
                });
            });
        }

        UpdateState::Idle | UpdateState::Checking => {}
    }

    section_row(ui, false, true, |ui| {
        ui.vertical(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new("Check for updates on startup")
                    .size(12.5)
                    .color(C_TEXT),
            );
            ui.label(
                RichText::new("Queries GitHub Releases once on every launch")
                    .size(10.5)
                    .color(C_TEXT_MUTED),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut val = app.config.auto_update_check;
            if toggle(ui, Id::new("toggle_auto_update"), &mut val) {
                app.config.auto_update_check = val;
                let _ = app.config.save();
            }
        });
    });
}
