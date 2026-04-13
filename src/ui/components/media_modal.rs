use crate::core::models::{
    AutocompleteData, MediaItem, MediaModalMode, MediaType, ModalAction, StagingItem,
};
use crate::ui::app::MediaApp;
use crate::ui::colors::{
    BACKDROP, BORDER, CARD_BG, C_BLURPLE, C_HOVER, C_INPUT_BG, C_TEXT, C_TEXT_HEADER, C_TEXT_MUTED,
    DANGER, SECTION_BG,
};
use crate::ui::components::widgets::pill_button::pill_button;
use crate::ui::icon_registry::IconRegistry;
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Image, Margin, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, Vec2,
};
use std::sync::Arc;

const MODAL_W: f32 = 500.0;
const INNER_W: f32 = MODAL_W - 40.0;

const TAG_H: f32 = 26.0;
const TAG_FONT: f32 = 11.5;
const TAG_PAD_X: f32 = 10.0;
const TAG_GAP: f32 = 6.0;
const X_ZONE_W: f32 = 20.0;
const CHIP_CR: f32 = 5.0;
const ROW_H: f32 = 27.0;
const MAX_SUGG: usize = 8;

#[derive(Default)]
pub struct MediaModalState {
    pub mode: Option<MediaModalMode>,

    pub copyright: String,
    pub artist: String,
    pub characters: Vec<String>,
    pub chars_input: String,
    pub tags: Vec<String>,
    pub tags_input: String,
    pub video_title: String,

    pub copyright_suggestions: Vec<String>,
    pub artist_suggestions: Vec<String>,
    pub char_suggestions: Vec<String>,
    pub tag_suggestions: Vec<String>,

    pub copyright_popup: bool,
    pub artist_popup: bool,
    pub chars_popup: bool,
    pub tags_popup: bool,

    pub error: Option<String>,
}

impl MediaModalState {
    pub fn open_edit(&mut self, item: Arc<MediaItem>, autocomplete: &AutocompleteData) {
        let copyright = item.copyright.clone();
        let artist = item.artist.clone();
        let characters = item.characters.clone();
        let tags = item.tags.clone();
        *self = Self::default();
        self.copyright = copyright;
        self.artist = artist;
        self.characters = characters;
        self.tags = tags;
        self.copyright_suggestions = autocomplete.copyrights.clone();
        self.artist_suggestions = autocomplete.artists.clone();
        self.char_suggestions = autocomplete.characters.clone();
        self.tag_suggestions = autocomplete.tags.clone();
        self.mode = Some(MediaModalMode::Edit(item));
    }

    pub fn open_distribute(&mut self, item: Arc<StagingItem>, autocomplete: &AutocompleteData) {
        *self = Self::default();
        self.copyright_suggestions = autocomplete.copyrights.clone();
        self.artist_suggestions = autocomplete.artists.clone();
        self.char_suggestions = autocomplete.characters.clone();
        self.tag_suggestions = autocomplete.tags.clone();
        self.mode = Some(MediaModalMode::Distribute(item));
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn is_open(&self) -> bool {
        self.mode.is_some()
    }

    pub fn is_valid(&self) -> bool {
        !self.copyright.trim().is_empty() && !self.artist.trim().is_empty()
    }
}

fn filter_suggestions(list: &[String], query: &str) -> Vec<String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    list.iter()
        .filter(|s| s.to_lowercase().contains(&q))
        .cloned()
        .take(MAX_SUGG)
        .collect()
}

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
    let mut x_resp = ui.interact(x_rect, ui.id().with(label), Sense::click());
    x_resp = x_resp.on_hover_cursor(CursorIcon::PointingHand);

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
        let icon_rect = Rect::from_center_size(x_rect.center(), Vec2::splat(12.0));
        let tint = if x_resp.hovered() {
            Color32::WHITE
        } else {
            C_TEXT_MUTED
        };
        ui.painter().image(
            icons.get("close").id(),
            icon_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
    }
    x_resp.clicked()
}

fn dropdown(
    ui: &egui::Ui,
    popup_id: Id,
    anchor: Pos2,
    width: f32,
    suggestions: &[String],
    exclude: &[String],
) -> (Option<usize>, bool) {
    let visible: Vec<(usize, &String)> = suggestions
        .iter()
        .enumerate()
        .filter(|(_, s)| !exclude.contains(s))
        .take(MAX_SUGG)
        .collect();

    if visible.is_empty() {
        return (None, false);
    }

    let mut clicked: Option<usize> = None;

    let area = egui::Area::new(popup_id)
        .fixed_pos(anchor)
        .order(egui::Order::Tooltip)
        .show(ui.ctx(), |ui| {
            Frame::NONE
                .fill(C_INPUT_BG)
                .corner_radius(CornerRadius::same(8))
                .stroke(Stroke::new(1.0, BORDER))
                .inner_margin(Margin::same(4))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 12,
                    spread: 0,
                    color: Color32::from_black_alpha(80),
                })
                .show(ui, |ui| {
                    ui.set_width(width - 8.0);
                    for (orig_idx, label) in &visible {
                        let (r, mut resp) =
                            ui.allocate_exact_size(Vec2::new(width - 8.0, ROW_H), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, 5.0, C_HOVER);
                            resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                        }
                        ui.painter().text(
                            Pos2::new(r.min.x + 10.0, r.center().y),
                            egui::Align2::LEFT_CENTER,
                            label,
                            FontId::proportional(12.5),
                            if resp.hovered() {
                                C_TEXT_HEADER
                            } else {
                                C_TEXT
                            },
                        );
                        if resp.clicked() {
                            clicked = Some(*orig_idx);
                        }
                    }
                });
        });

    let ptr = ui
        .ctx()
        .input(|i| i.pointer.interact_pos())
        .unwrap_or_default();
    let outside = ui.ctx().input(|i| i.pointer.any_click()) && !area.response.rect.contains(ptr);
    (clicked, outside)
}

fn autocomplete_single(
    ui: &mut egui::Ui,
    value: &mut String,
    suggestions: &[String],
    popup_open: &mut bool,
    hint: &str,
    popup_id: Id,
) -> bool {
    let mut changed = false;

    ui.allocate_ui_with_layout(
        egui::vec2(INNER_W, 42.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            Frame::NONE
                .fill(SECTION_BG)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(12, 12))
                .stroke(Stroke::new(1.0, BORDER))
                .show(ui, |ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(value)
                            .hint_text(hint)
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .text_color(C_TEXT),
                    );
                    if resp.changed() {
                        changed = true;
                        *popup_open = !value.trim().is_empty();
                    }
                });

            if *popup_open && !suggestions.is_empty() {
                let anchor = Pos2::new(ui.min_rect().min.x, ui.min_rect().max.y + 2.0);
                let (clicked, outside) = dropdown(ui, popup_id, anchor, INNER_W, suggestions, &[]);
                if let Some(idx) = clicked {
                    *value = suggestions[idx].clone();
                    *popup_open = false;
                    changed = true;
                } else if outside {
                    *popup_open = false;
                }
                ui.add_space((suggestions.len().min(MAX_SUGG) as f32 * ROW_H + 12.0).min(240.0));
            }
        },
    );

    changed
}

fn chip_section_autocomplete(
    ui: &mut egui::Ui,
    items: &mut Vec<String>,
    input: &mut String,
    suggestions: &[String],
    popup_open: &mut bool,
    accent: Color32,
    hint: &str,
    empty_label: &str,
    popup_id: Id,
    icons: &IconRegistry,
) -> bool {
    let mut changed = false;

    let mut remove_idx: Option<usize> = None;
    if items.is_empty() {
        ui.style_mut().interaction.selectable_labels = false;
        ui.label(RichText::new(empty_label).size(11.5).color(C_TEXT_MUTED));
        ui.add_space(8.0);
    } else {
        let snap = items.clone();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(TAG_GAP);
            for (i, item) in snap.iter().enumerate() {
                if chip(ui, item, accent, icons) {
                    remove_idx = Some(i);
                }
            }
        });
        ui.add_space(10.0);
    }
    if let Some(idx) = remove_idx {
        items.remove(idx);
        changed = true;
    }

    let mut add_pending: Option<String> = None;

    ui.allocate_ui_with_layout(
        egui::vec2(INNER_W, 42.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            Frame::NONE
                .fill(SECTION_BG)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(12, 12))
                .stroke(Stroke::new(1.0, BORDER))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let t = input.trim().to_owned();
                        let can_add = !t.is_empty() && !items.contains(&t);
                        let add_clicked = pill_button(ui, "Add", can_add);

                        let field = ui.add(
                            egui::TextEdit::singleline(input)
                                .id(popup_id.with("__field"))
                                .hint_text(hint)
                                .frame(Frame::NONE)
                                .desired_width(INNER_W)
                                .text_color(C_TEXT),
                        );

                        if field.changed() {
                            changed = true;
                            *popup_open = !input.trim().is_empty();
                        }

                        let enter =
                            field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                        let final_t = input.trim().to_owned();
                        let can_final = !final_t.is_empty() && !items.contains(&final_t);

                        if (add_clicked || enter) && can_final {
                            add_pending = Some(final_t);
                        }
                    });
                });

            if let Some(v) = add_pending {
                items.push(v);
                input.clear();
                *popup_open = false;
                changed = true;
            }

            if *popup_open && !suggestions.is_empty() {
                let anchor = Pos2::new(ui.min_rect().min.x, ui.min_rect().max.y + 2.0);
                let (clicked, outside) = dropdown(
                    ui,
                    popup_id.with("__drop"),
                    anchor,
                    INNER_W,
                    suggestions,
                    items,
                );
                if let Some(idx) = clicked {
                    items.push(suggestions[idx].clone());
                    input.clear();
                    *popup_open = false;
                    changed = true;
                } else if outside {
                    *popup_open = false;
                }
                ui.add_space((suggestions.len().min(MAX_SUGG) as f32 * ROW_H + 12.0).min(240.0));
            }
        },
    );

    changed
}

fn close_button(ui: &mut egui::Ui, icons: &IconRegistry) -> bool {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(
                rect,
                7.0,
                Color32::from_rgba_premultiplied(255, 255, 255, 12),
            );
        }
        ui.put(
            Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
            Image::new(icons.get("close"))
                .fit_to_exact_size(Vec2::splat(16.0))
                .tint(C_TEXT_MUTED),
        );
    }
    resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    resp.clicked()
}

pub fn media_modal(app: &mut MediaApp, ui: &egui::Ui) -> ModalAction {
    if !app.modal_state.is_open() {
        return ModalAction::None;
    }

    let ctx = ui.ctx();
    let screen = ctx.content_rect();
    let mut action = ModalAction::None;
    let icons = app.icons.as_ref().unwrap();

    let (title, is_distribute, is_video) = match &app.modal_state.mode {
        Some(MediaModalMode::Edit(_)) => ("Edit Metadata", false, false),
        Some(MediaModalMode::Distribute(item)) => (
            "Distribute to Library",
            true,
            matches!(item.media_type, MediaType::Video),
        ),
        None => return ModalAction::None,
    };

    egui::Area::new(Id::new("media_modal_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            let resp = ui.allocate_rect(screen, Sense::click());
            ui.painter().rect_filled(screen, 0.0, BACKDROP);
            if resp.clicked() {
                action = ModalAction::Close;
            }
        });

    egui::Window::new("##media_modal")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_size([MODAL_W, 0.0])
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
            ui.set_width(MODAL_W);

            Frame::NONE
                .inner_margin(Margin::symmetric(20, 0))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(INNER_W, 52.0));

                    ui.horizontal(|ui| {
                        ui.set_min_height(52.0);
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(
                            RichText::new(title)
                                .size(16.0)
                                .color(C_TEXT_HEADER)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if close_button(ui, icons) {
                                action = ModalAction::Close;
                            }
                        });
                    });

                    ui.style_mut().interaction.selectable_labels = false;
                    match &app.modal_state.mode {
                        Some(MediaModalMode::Edit(item)) => {
                            let name = if item.name.len() > 56 {
                                format!("…{}", &item.name[item.name.len() - 54..])
                            } else {
                                item.name.clone()
                            };
                            ui.label(RichText::new(name).size(10.5).color(C_TEXT_MUTED));
                        }
                        Some(MediaModalMode::Distribute(item)) => {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&item.path).size(10.0).color(C_TEXT_MUTED),
                                )
                                .wrap(),
                            );
                        }
                        None => {}
                    }
                    ui.add_space(8.0);
                });

            let (sep, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
            ui.painter().rect_filled(sep, 0.0, BORDER);

            Frame::NONE
                .inner_margin(Margin::symmetric(20, 16))
                .show(ui, |ui| {
                    ui.set_width(INNER_W);

                    ui.style_mut().interaction.selectable_labels = false;
                    ui.label(RichText::new("COPYRIGHT").size(10.5).color(C_TEXT_MUTED));
                    ui.add_space(6.0);
                    let cr_sug = app.modal_state.copyright_suggestions.clone();
                    if autocomplete_single(
                        ui,
                        &mut app.modal_state.copyright,
                        &cr_sug,
                        &mut app.modal_state.copyright_popup,
                        "e.g. Marvel, Studio Ghibli…",
                        Id::new("mm_cr"),
                    ) {
                        app.modal_state.copyright_suggestions = filter_suggestions(
                            &app.autocomplete.copyrights,
                            &app.modal_state.copyright,
                        );
                    }
                    ui.add_space(14.0);

                    ui.label(RichText::new("ARTIST").size(10.5).color(C_TEXT_MUTED));
                    ui.add_space(6.0);
                    let ar_sug = app.modal_state.artist_suggestions.clone();
                    if autocomplete_single(
                        ui,
                        &mut app.modal_state.artist,
                        &ar_sug,
                        &mut app.modal_state.artist_popup,
                        "Artist / creator name…",
                        Id::new("mm_ar"),
                    ) {
                        app.modal_state.artist_suggestions =
                            filter_suggestions(&app.autocomplete.artists, &app.modal_state.artist);
                    }
                    ui.add_space(14.0);

                    ui.label(RichText::new("CHARACTERS").size(10.5).color(C_TEXT_MUTED));
                    ui.add_space(6.0);
                    let ch_sug = app.modal_state.char_suggestions.clone();
                    if chip_section_autocomplete(
                        ui,
                        &mut app.modal_state.characters,
                        &mut app.modal_state.chars_input,
                        &ch_sug,
                        &mut app.modal_state.chars_popup,
                        Color32::from_rgb(140, 100, 230),
                        "Add a character…",
                        "No characters — add one below.",
                        Id::new("mm_chars"),
                        icons,
                    ) {
                        app.modal_state.char_suggestions = filter_suggestions(
                            &app.autocomplete.characters,
                            &app.modal_state.chars_input,
                        );
                    }
                    ui.add_space(14.0);

                    ui.label(RichText::new("TAGS").size(10.5).color(C_TEXT_MUTED));
                    ui.add_space(6.0);
                    let tg_sug = app.modal_state.tag_suggestions.clone();
                    if chip_section_autocomplete(
                        ui,
                        &mut app.modal_state.tags,
                        &mut app.modal_state.tags_input,
                        &tg_sug,
                        &mut app.modal_state.tags_popup,
                        C_BLURPLE,
                        "Add a tag…",
                        "No tags — add one below.",
                        Id::new("mm_tags"),
                        icons,
                    ) {
                        app.modal_state.tag_suggestions =
                            filter_suggestions(&app.autocomplete.tags, &app.modal_state.tags_input);
                    }
                    ui.add_space(14.0);

                    if is_distribute && is_video {
                        ui.label(
                            RichText::new("VIDEO TITLE (optional)")
                                .size(10.5)
                                .color(C_TEXT_MUTED),
                        );
                        ui.add_space(6.0);
                        Frame::NONE
                            .fill(SECTION_BG)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(12, 0))
                            .stroke(Stroke::new(1.0, BORDER))
                            .show(ui, |ui| {
                                ui.set_width(INNER_W);
                                ui.horizontal(|ui| {
                                    ui.set_min_height(42.0);
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut app.modal_state.video_title,
                                        )
                                        .hint_text("Leave empty to use original filename…")
                                        .frame(Frame::NONE)
                                        .desired_width(f32::INFINITY)
                                        .text_color(C_TEXT),
                                    );
                                });
                            });
                        ui.add_space(14.0);
                    }

                    if let Some(err) = &app.modal_state.error.clone() {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("⚠ {err}")).size(11.0).color(DANGER),
                            )
                            .wrap(),
                        );
                        ui.add_space(10.0);
                    }

                    let (fsep, _) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().rect_filled(fsep, 0.0, BORDER);
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if pill_button(ui, "Cancel", true) {
                            action = ModalAction::Close;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ok = app.modal_state.is_valid();
                            let label = if is_distribute { "Distribute" } else { "Save" };
                            if pill_button(ui, label, ok) && ok {
                                action = if is_distribute {
                                    ModalAction::Distribute
                                } else {
                                    ModalAction::SaveEdit
                                };
                            }
                        });
                    });
                    ui.add_space(4.0);
                });
        });

    action
}
