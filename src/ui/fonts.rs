use eframe::epaint::text::{FontData, FontDefinitions};
use eframe::epaint::FontFamily;
use std::sync::Arc;

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // основной шрифт
    fonts.font_data.insert(
        "inter".to_owned(),
        Arc::from(FontData::from_static(include_bytes!(
            "../../assets/fonts/Inter-Regular.ttf"
        ))),
    );

    // emoji
    fonts.font_data.insert(
        "emoji".to_owned(),
        Arc::from(FontData::from_static(include_bytes!(
            "../../assets/fonts/NotoColorEmoji-Regular.ttf"
        ))),
    );

    // ставим порядок (ВАЖНО)
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "inter".to_owned());

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .push("emoji".to_owned());

    // monospace (опционально)
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .push("emoji".to_owned());

    ctx.set_fonts(fonts);
}
