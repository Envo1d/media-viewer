use crate::core::models::UpdateState;
use crate::infra::updater::current_version;
use crate::ui::app::MediaApp;
use crate::ui::colors::{C_TEXT, C_TEXT_MUTED, DANGER};
use crate::ui::components::widgets::pill_button::pill_button;
use crate::ui::components::widgets::section_heading::section_heading;
use crate::ui::components::widgets::section_row::section_row;
use crate::ui::components::widgets::toggle::toggle;
use egui::{Id, RichText};

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
            let can_check = !matches!(app.update_state, UpdateState::Checking);
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
                        RichText::new(format!(
                            "Download size: {}  ·  Verified with Ed25519",
                            bytes_human(size_bytes)
                        ))
                        .size(10.5)
                        .color(C_TEXT_MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, "Download & Install", true) {
                        app.apply_update();
                    }
                });
            });
        }

        UpdateState::Downloading { .. } | UpdateState::ReadyToInstall { .. } => {}

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
