use crate::ui::app::MediaApp;
use crate::ui::components::media_card::media_card;
use eframe::emath::Vec2;
use egui::Ui;

pub fn grid_layout(app: &mut MediaApp, ui: &mut Ui) {
    // data
    let items = &app.displayed_items;

    // params
    let item_size = 200.0;
    let spacing = 10.0;
    let available_width = ui.available_width() * 0.8;
    let columns = ((available_width + spacing) / (item_size + spacing))
        .floor()
        .max(1.0) as usize;
    let total_width = columns as f32 * item_size + (columns - 1) as f32 * spacing;
    let side_padding = ((ui.available_width() - total_width) / 2.0).max(0.0);
    let row_height = item_size + spacing;
    let total_rows = (items.len() + columns - 1) / columns;

    egui::ScrollArea::vertical()
        .animated(true)
        .wheel_scroll_multiplier(Vec2::new(2.0, 2.0))
        .show_rows(ui, row_height, total_rows, |ui, row_range| {
            let margin = 1;
            let prefetch_rows = (row_range.start.saturating_sub(margin)..row_range.start)
                .chain(row_range.end..(row_range.end + margin).min(total_rows));

            for p_row in prefetch_rows {
                for col in 0..columns {
                    let index = p_row * columns + col;
                    if let Some(item) = items.get(index) {
                        app.texture_manager.prefetch(&item.path);
                    }
                }
            }

            ui.add_space(spacing);

            for row in row_range {
                ui.horizontal(|ui| {
                    ui.add_space(side_padding);

                    for col in 0..columns {
                        let index = row * columns + col;
                        if index >= items.len() {
                            break;
                        }

                        if let Some(item) = items.get(index) {
                            media_card(ui, item, &mut app.texture_manager, item_size);
                        }

                        if col < columns - 1 {
                            ui.add_space(spacing);
                        }
                    }
                    ui.add_space(side_padding);
                });
            }
            ui.add_space(spacing);
        });
}
