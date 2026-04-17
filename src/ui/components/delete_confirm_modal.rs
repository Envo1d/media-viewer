use crate::core::models::PendingDelete;
use crate::ui::app::MediaApp;
use crate::ui::colors::{C_BLURPLE, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED, DANGER};
use crate::ui::components::modal_window::{modal_backdrop, modal_frame_window, modal_separator};
use eframe::emath::Vec2;
use eframe::epaint::{Color32, Margin};
use egui::{Frame, RichText, Sense};
use std::path::Path;

const MODAL_W: f32 = 420.0;

pub fn delete_confirm_modal(app: &mut MediaApp, ui: &egui::Ui) {
    let Some(pending) = &app.pending_delete else {
        return;
    };

    let (filename, path_preview) = match pending {
        PendingDelete::Library(item) => (item.name.clone(), item.path.clone()),
        PendingDelete::Staging(item) => (item.name.clone(), item.path.clone()),
    };

    let dir_path = Path::new(&path_preview)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let shown_path = if dir_path.len() > 60 {
        format!("…{}", &dir_path[dir_path.len() - 58..])
    } else {
        dir_path
    };

    let ctx = ui.ctx();

    let mut confirmed = false;
    let mut cancelled = false;

    if modal_backdrop(ctx, "delete_confirm_backdrop", egui::Order::Middle) {
        cancelled = true;
    }

    modal_frame_window("##delete_confirm", MODAL_W, None).show(ctx, |ui| {
        Frame::NONE
            .inner_margin(Margin::symmetric(24, 24))
            .show(ui, |ui| {
                ui.set_width(372.0);
                ui.style_mut().interaction.selectable_labels = false;

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Delete file?")
                            .size(16.0)
                            .color(C_TEXT_HEADER)
                            .strong(),
                    );
                });
                ui.add_space(10.0);

                ui.label(RichText::new(&filename).size(12.5).color(C_TEXT));
                ui.add_space(4.0);

                ui.add(
                    egui::Label::new(RichText::new(&shown_path).size(10.5).color(C_TEXT_MUTED))
                        .wrap(),
                );
                ui.add_space(6.0);

                ui.add(
                    egui::Label::new(
                        RichText::new("This will permanently delete the file from disk.")
                            .size(11.0)
                            .color(DANGER),
                    )
                    .wrap(),
                );
                ui.add_space(20.0);

                modal_separator(ui);

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let cancel_galley = ui.fonts_mut(|f| {
                        f.layout_no_wrap(
                            "Cancel".to_owned(),
                            egui::FontId::proportional(12.0),
                            C_TEXT_HEADER,
                        )
                    });
                    let cancel_size = Vec2::new(
                        cancel_galley.rect.width() + 28.0,
                        cancel_galley.rect.height() + 10.0,
                    );
                    let (cancel_rect, mut cancel_resp) =
                        ui.allocate_exact_size(cancel_size, Sense::click());
                    if ui.is_rect_visible(cancel_rect) {
                        let fill = if cancel_resp.is_pointer_button_down_on() {
                            C_BLURPLE.linear_multiply(0.70)
                        } else if cancel_resp.hovered() {
                            C_BLURPLE.linear_multiply(0.85)
                        } else {
                            C_BLURPLE
                        };
                        ui.painter().rect_filled(cancel_rect, 6.0, fill);
                        ui.painter().galley(
                            cancel_rect.min + Vec2::new(14.0, 5.0),
                            cancel_galley,
                            C_TEXT_HEADER,
                        );
                    }
                    if cancel_resp.hovered() {
                        cancel_resp = cancel_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    }
                    if cancel_resp.clicked() {
                        cancelled = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let del_galley = ui.fonts_mut(|f| {
                            f.layout_no_wrap(
                                "Delete".to_owned(),
                                egui::FontId::proportional(12.0),
                                Color32::WHITE,
                            )
                        });
                        let del_size = Vec2::new(
                            del_galley.rect.width() + 28.0,
                            del_galley.rect.height() + 10.0,
                        );
                        let (del_rect, mut del_resp) =
                            ui.allocate_exact_size(del_size, Sense::click());
                        if ui.is_rect_visible(del_rect) {
                            let fill = if del_resp.is_pointer_button_down_on() {
                                DANGER.linear_multiply(0.70)
                            } else if del_resp.hovered() {
                                DANGER.linear_multiply(1.15)
                            } else {
                                DANGER
                            };
                            ui.painter().rect_filled(del_rect, 6.0, fill);
                            ui.painter().galley(
                                del_rect.min + Vec2::new(14.0, 5.0),
                                del_galley,
                                Color32::WHITE,
                            );
                        }
                        if del_resp.hovered() {
                            del_resp = del_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        }
                        if del_resp.clicked() {
                            confirmed = true;
                        }
                    });
                });
            });
    });

    if confirmed {
        let pending = app.pending_delete.take().unwrap();
        match pending {
            PendingDelete::Library(item) => app.do_delete_library(item),
            PendingDelete::Staging(item) => app.do_delete_staging(item),
        }
    } else if cancelled {
        app.pending_delete = None;
    }
}
