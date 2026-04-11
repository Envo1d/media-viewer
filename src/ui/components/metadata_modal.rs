use crate::ui::app::MediaApp;
use crate::ui::colors::{
    BACKDROP, BORDER, CARD_BG, C_BLURPLE, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED, SECTION_BG,
};
use crate::ui::components::widgets::pill_button::pill_button;
use crate::ui::icon_registry::IconRegistry;
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Image, Margin, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, Vec2,
};

const MODAL_W: f32 = 440.0;

// Chip geometry
const TAG_H: f32 = 26.0;
const TAG_FONT: f32 = 11.5;
const TAG_PAD_X: f32 = 10.0;
const TAG_GAP: f32 = 6.0;
const X_ZONE_W: f32 = 20.0;
const CHIP_CR: f32 = 5.0;

fn chip(ui: &mut egui::Ui, label: &str, accent: Color32, icons: &IconRegistry) -> bool {
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            label.to_owned(),
            FontId::proportional(TAG_FONT),
            Color32::WHITE,
        )
    });

    let chip_w = galley.rect.width() + TAG_PAD_X * 2.0 + X_ZONE_W;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(chip_w, TAG_H), Sense::hover());

    let x_rect = Rect::from_min_size(
        Pos2::new(rect.max.x - X_ZONE_W, rect.min.y),
        Vec2::new(X_ZONE_W, TAG_H),
    );
    let x_resp = ui.interact(x_rect, ui.id().with(label), Sense::click());

    if x_resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let bg = if x_resp.hovered() {
            accent.linear_multiply(0.55)
        } else {
            accent.linear_multiply(0.30)
        };

        ui.painter().rect_filled(rect, CHIP_CR, bg);
        ui.painter().rect_stroke(
            rect,
            CHIP_CR,
            Stroke::new(1.0, accent.linear_multiply(0.65)),
            StrokeKind::Outside,
        );

        let text_y = rect.center().y - galley.rect.height() / 2.0;
        ui.painter().galley(
            Pos2::new(rect.min.x + TAG_PAD_X, text_y),
            galley,
            C_TEXT_HEADER,
        );

        let icon = icons.get("close");

        let icon_size = 12.0;
        let icon_rect = Rect::from_center_size(x_rect.center(), Vec2::splat(icon_size));

        let tint = if x_resp.hovered() {
            Color32::WHITE
        } else {
            C_TEXT_MUTED
        };

        ui.painter().image(
            icon.id(),
            icon_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
    }

    x_resp.clicked()
}

fn chip_section(
    ui: &mut egui::Ui,
    items: &mut Vec<String>,
    input: &mut String,
    accent: Color32,
    hint: &str,
    empty_label: &str,
    icons: &IconRegistry,
) {
    let mut remove_idx: Option<usize> = None;
    if items.is_empty() {
        ui.label(RichText::new(empty_label).size(11.5).color(C_TEXT_MUTED));
        ui.add_space(8.0);
    } else {
        let items_snap = items.clone();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(TAG_GAP);
            for (i, item) in items_snap.iter().enumerate() {
                if chip(ui, item, accent, icons) {
                    remove_idx = Some(i);
                }
            }
        });
        ui.add_space(10.0);
    }
    if let Some(idx) = remove_idx {
        items.remove(idx);
    }

    Frame::NONE
        .fill(SECTION_BG)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 0))
        .stroke(Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(MODAL_W - 40.0, 42.0));
            ui.horizontal(|ui| {
                ui.set_min_height(42.0);

                let input_resp = ui.add(
                    egui::TextEdit::singleline(input)
                        .hint_text(hint)
                        .frame(Frame::NONE)
                        .desired_width(f32::INFINITY)
                        .text_color(C_TEXT),
                );

                let pressed_enter =
                    input_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let trimmed = input.trim().to_owned();
                    let can_add = !trimmed.is_empty() && !items.iter().any(|t| t == &trimmed);

                    if (pill_button(ui, "Add", can_add) || pressed_enter) && can_add {
                        items.push(trimmed);
                        input.clear();
                        input_resp.request_focus();
                    }
                });
            });
        });
}

pub fn metadata_modal(app: &mut MediaApp, ui: &egui::Ui) {
    if app.metadata_modal_item.is_none() {
        return;
    }

    let ctx = ui.ctx();
    let screen = ctx.content_rect();
    let mut close = false;
    let mut save_requested = false;

    let icons = app.icons.as_ref().unwrap();

    egui::Area::new(Id::new("metadata_modal_backdrop"))
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

    egui::Window::new("##metadata_modal")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
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
            ui.set_min_width(MODAL_W);
            ui.set_max_width(MODAL_W);

            Frame::NONE
                .inner_margin(Margin::symmetric(20, 0))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(MODAL_W - 40.0, 48.0));

                    ui.horizontal(|ui| {
                        ui.set_min_height(48.0);

                        ui.label(
                            RichText::new("Edit Metadata")
                                .size(16.0)
                                .color(C_TEXT_HEADER)
                                .strong(),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (btn_rect, btn_resp) =
                                ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                            if ui.is_rect_visible(btn_rect) {
                                if btn_resp.hovered() {
                                    ui.painter().rect_filled(
                                        btn_rect,
                                        7.0,
                                        Color32::from_rgba_premultiplied(255, 255, 255, 14),
                                    );
                                }
                                let close_icon = app.icons.as_ref().unwrap().get("close");
                                let icon_rect =
                                    Rect::from_center_size(btn_rect.center(), Vec2::splat(14.0));
                                ui.put(
                                    icon_rect,
                                    Image::new(close_icon)
                                        .fit_to_exact_size(Vec2::splat(14.0))
                                        .tint(C_TEXT_MUTED),
                                );
                            }
                            if btn_resp.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            }
                            if btn_resp.clicked() {
                                close = true;
                            }
                        });
                    });

                    if let Some(item) = &app.metadata_modal_item {
                        let name = if item.name.len() > 50 {
                            format!("…{}", &item.name[item.name.len() - 48..])
                        } else {
                            item.name.clone()
                        };
                        ui.label(RichText::new(name).size(10.5).color(C_TEXT_MUTED));
                    }
                    ui.add_space(8.0);
                });

            let (sep, _) = ui.allocate_exact_size(Vec2::new(MODAL_W, 1.0), Sense::hover());
            ui.painter().rect_filled(sep, 0.0, BORDER);

            Frame::NONE
                .inner_margin(Margin::symmetric(20, 16))
                .show(ui, |ui| {
                    ui.set_width(MODAL_W - 40.0);

                    ui.label(RichText::new("CHARACTERS").size(10.5).color(C_TEXT_MUTED));
                    ui.add_space(6.0);

                    let char_accent = Color32::from_rgb(140, 100, 230);
                    chip_section(
                        ui,
                        &mut app.metadata_modal_chars,
                        &mut app.metadata_modal_chars_input,
                        char_accent,
                        "Add a character…",
                        "No characters – add one below.",
                        icons,
                    );

                    ui.add_space(14.0);

                    ui.label(RichText::new("TAGS").size(10.5).color(C_TEXT_MUTED));
                    ui.add_space(6.0);

                    chip_section(
                        ui,
                        &mut app.metadata_modal_tags,
                        &mut app.metadata_modal_input,
                        C_BLURPLE,
                        "Add a tag…",
                        "No tags – add one below.",
                        icons,
                    );

                    ui.add_space(16.0);

                    let (fsep, _) =
                        ui.allocate_exact_size(Vec2::new(MODAL_W - 40.0, 1.0), Sense::hover());
                    ui.painter().rect_filled(fsep, 0.0, BORDER);
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if pill_button(ui, "Cancel", true) {
                            close = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if pill_button(ui, "Save", true) {
                                save_requested = true;
                            }
                        });
                    });

                    ui.add_space(4.0);
                });
        });

    if save_requested {
        app.save_metadata();
    } else if close {
        app.metadata_modal_item = None;
        app.metadata_modal_chars.clear();
        app.metadata_modal_chars_input.clear();
        app.metadata_modal_tags.clear();
        app.metadata_modal_input.clear();
    }
}
