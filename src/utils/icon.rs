pub fn icon(ui: &mut egui::Ui, tex: &egui::TextureHandle, size: f32) {
    ui.add(egui::Image::new(tex).fit_to_exact_size(egui::Vec2::splat(size)));
}
