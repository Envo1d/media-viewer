use crate::ui::colors::{BACKDROP, BORDER, CARD_BG, C_TEXT_HEADER, C_TEXT_MUTED};
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, Frame, Id, Image, Margin, Pos2, Rect, RichText,
    Sense, Stroke, Vec2,
};

pub fn modal_backdrop(ctx: &egui::Context, id: impl std::hash::Hash, order: egui::Order) -> bool {
    let screen = ctx.content_rect();
    let mut clicked = false;
    egui::Area::new(Id::new(id))
        .fixed_pos(Pos2::ZERO)
        .order(order)
        .interactable(true)
        .show(ctx, |ui| {
            let resp = ui.allocate_rect(screen, Sense::click());
            ui.painter().rect_filled(screen, 0.0, BACKDROP);
            if resp.clicked() {
                clicked = true;
            }
        });
    clicked
}

pub fn modal_frame_window(
    id: &'static str,
    width: f32,
    fixed_height: Option<f32>,
) -> egui::Window<'static> {
    let frame = Frame::NONE
        .fill(CARD_BG)
        .corner_radius(CornerRadius::same(14))
        .stroke(Stroke::new(1.0, BORDER))
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 40,
            spread: 0,
            color: Color32::from_black_alpha(120),
        });

    let size = match fixed_height {
        Some(h) => [width, h],
        None => [width, 0.0],
    };

    egui::Window::new(id)
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_size(size)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(frame)
}

pub fn modal_header(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<String>,
    height: f32,
    close_icon: &egui::TextureHandle,
) -> bool {
    let mut close_clicked = false;
    let inner_w = ui.available_width() - 40.0;
    Frame::NONE
        .inner_margin(Margin::symmetric(20, 0))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(inner_w, height));
            ui.horizontal(|ui| {
                ui.set_min_height(height);
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                    RichText::new(title)
                        .size(16.0)
                        .color(C_TEXT_HEADER)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if modal_close_button(ui, close_icon) {
                        close_clicked = true;
                    }
                });
            });

            if let Some(s) = subtitle {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(RichText::new(s).size(10.5).color(C_TEXT_MUTED));
                ui.add_space(8.0);
            }
        });
    close_clicked
}

pub fn modal_close_button(ui: &mut egui::Ui, close_icon: &egui::TextureHandle) -> bool {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(
                rect,
                7.0,
                Color32::from_rgba_premultiplied(255, 255, 255, 12),
            );
        }
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(16.0));
        ui.put(
            icon_rect,
            Image::new(close_icon)
                .fit_to_exact_size(Vec2::splat(16.0))
                .tint(C_TEXT_MUTED),
        );
    }
    resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    resp.clicked()
}

pub fn modal_separator(ui: &mut egui::Ui) {
    let (sep, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(sep, 0.0, BORDER);
}
