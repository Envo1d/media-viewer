use crate::ui::colors::{
    C_BLURPLE, C_HOVER, C_INPUT_BG, C_SECONDARY_BG, C_SELECTED, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED,
};

pub fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();

    style.spacing.item_spacing = egui::Vec2::new(0.0, 0.0);
    style.spacing.window_margin = egui::Margin::same(0);
    style.spacing.scroll = egui::style::ScrollStyle {
        bar_width: 6.0,
        floating: true,
        ..Default::default()
    };

    style.visuals.window_fill = C_SECONDARY_BG;
    style.visuals.panel_fill = C_SECONDARY_BG;
    style.visuals.extreme_bg_color = C_INPUT_BG;
    style.visuals.faint_bg_color = C_HOVER;

    style.visuals.widgets.noninteractive.bg_fill = C_SECONDARY_BG;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, C_TEXT);
    style.visuals.widgets.inactive.bg_fill = C_INPUT_BG;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, C_TEXT_MUTED);
    style.visuals.widgets.hovered.bg_fill = C_HOVER;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, C_TEXT);
    style.visuals.widgets.active.bg_fill = C_SELECTED;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, C_TEXT_HEADER);

    style.visuals.selection.bg_fill = C_BLURPLE;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, C_TEXT_HEADER);

    style.visuals.window_shadow = egui::Shadow::NONE;

    // Scrollbar colors
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;

    ctx.set_global_style(style);
}
