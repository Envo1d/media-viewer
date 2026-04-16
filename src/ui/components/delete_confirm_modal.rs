use crate::core::models::PendingDelete;
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    BACKDROP, BORDER, CARD_BG, C_BLURPLE, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED, DANGER,
};
use eframe::emath::{Align2, Pos2, Vec2};
use eframe::epaint::{Color32, CornerRadius, Margin, Stroke};
use egui::{Frame, Id, RichText, Sense};
use std::path::Path;

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
        .unwrap_or_else(|| "".to_string());

    let shown_path = if dir_path.len() > 60 {
        format!("…{}", &dir_path[dir_path.len() - 58..])
    } else {
        dir_path
    };

    let ctx = ui.ctx();
    let screen = ctx.content_rect();

    let mut confirmed = false;
    let mut cancelled = false;

    egui::Area::new(Id::new("delete_confirm_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            let resp = ui.allocate_rect(screen, Sense::click());
            ui.painter().rect_filled(screen, 0.0, BACKDROP);
            if resp.clicked() {
                cancelled = true;
            }
        });

    egui::Window::new("##delete_confirm")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_size([420.0, 0.0])
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
            Frame::NONE
                .inner_margin(Margin::symmetric(24, 24))
                .show(ui, |ui| {
                    ui.set_width(372.0);
                    ui.style_mut().interaction.selectable_labels = false;

                    // Title row
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Delete file?")
                                .size(16.0)
                                .color(C_TEXT_HEADER)
                                .strong(),
                        );
                    });
                    ui.add_space(10.0);

                    // Filename
                    ui.label(RichText::new(&filename).size(12.5).color(C_TEXT));
                    ui.add_space(4.0);

                    // Path preview
                    ui.add(
                        egui::Label::new(RichText::new(&shown_path).size(10.5).color(C_TEXT_MUTED))
                            .wrap(),
                    );
                    ui.add_space(6.0);

                    // Warning line
                    ui.add(
                        egui::Label::new(
                            RichText::new("This will permanently delete the file from disk.")
                                .size(11.0)
                                .color(DANGER),
                        )
                        .wrap(),
                    );
                    ui.add_space(20.0);

                    // Separator
                    let (sep, _) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().rect_filled(sep, 0.0, BORDER);
                    ui.add_space(14.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        // Cancel – left side
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
                            cancel_resp =
                                cancel_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        }
                        if cancel_resp.clicked() {
                            cancelled = true;
                        }

                        // Delete – right side
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
