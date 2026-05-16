use crate::protocol::UpdaterArgs;
use crate::ui::colors::*;
use crate::ui::window_effects::WindowEffects;
use crate::worker::{self, WorkerCmd, WorkerEvent};
use crossbeam_channel::{Receiver, Sender};
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Margin, PointerButton, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, Vec2,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

const WIN_W: f32 = 440.0;
const WIN_H: f32 = 300.0;
const TITLE_H: f32 = 36.0;
const CORNER_R: u8 = 12;
const BTN_W: f32 = 36.0;
const ICON_SZ: f32 = 12.0;

const DONE_COUNTDOWN: f32 = 3.0;

#[derive(Debug, Clone)]
enum Stage {
    WaitingForParent,
    Downloading { done_bytes: u64, total_bytes: u64 },
    Verifying,
    Applying,
    Done { started: Instant },
    Error { message: String },
}

pub struct UpdaterApp {
    args: UpdaterArgs,

    stage: Stage,
    cmd_tx: Sender<WorkerCmd>,
    event_rx: Receiver<WorkerEvent>,
    cancel: Arc<AtomicBool>,

    spinner_angle: f32,

    display_progress: f32,

    close_icon: Option<egui::TextureHandle>,
    app_icon: Option<egui::TextureHandle>,

    window_fx: WindowEffects,
}

impl UpdaterApp {
    pub fn new(cc: &eframe::CreationContext<'_>, args: UpdaterArgs) -> Self {
        setup_style(&cc.egui_ctx);
        setup_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let app_icon = {
            let bytes = include_bytes!("../../../nexa/assets/icons/icon.png");
            egui_extras::image::load_image_bytes(bytes).ok().map(|img| {
                cc.egui_ctx
                    .load_texture("app_icon", img, Default::default())
            })
        };

        let close_icon = {
            let bytes = include_bytes!("../../../nexa/assets/icons/close.svg");
            let mut opt = usvg::Options::default();
            opt.dpi = 96.0 * cc.egui_ctx.pixels_per_point();
            egui_extras::image::load_svg_bytes(bytes, &opt)
                .ok()
                .map(|img| {
                    cc.egui_ctx
                        .load_texture("close_icon", img, Default::default())
                })
        };

        let (cmd_tx, event_rx, cancel) = worker::spawn(args.clone());

        Self {
            args,
            stage: Stage::WaitingForParent,
            cmd_tx,
            event_rx,
            cancel,
            spinner_angle: 0.0,
            display_progress: 0.0,
            close_icon,
            app_icon,
            window_fx: WindowEffects::new(),
        }
    }

    fn poll_events(&mut self, ctx: &egui::Context) {
        for event in self.event_rx.try_iter() {
            match event {
                WorkerEvent::WaitingForParent => {
                    self.stage = Stage::WaitingForParent;
                }
                WorkerEvent::DownloadStarted { total_bytes } => {
                    self.stage = Stage::Downloading {
                        done_bytes: 0,
                        total_bytes,
                    };
                    self.display_progress = 0.0;
                }
                WorkerEvent::DownloadProgress {
                    done_bytes,
                    total_bytes,
                } => {
                    self.stage = Stage::Downloading {
                        done_bytes,
                        total_bytes,
                    };
                }
                WorkerEvent::Verifying => {
                    self.stage = Stage::Verifying;
                }
                WorkerEvent::Applying => {
                    self.stage = Stage::Applying;
                }
                WorkerEvent::Done => {
                    self.display_progress = 1.0;
                    self.stage = Stage::Done {
                        started: Instant::now(),
                    };
                }
                WorkerEvent::Error { message } => {
                    self.stage = Stage::Error { message };
                }
            }
            ctx.request_repaint();
        }
    }
}

impl eframe::App for UpdaterApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.window_fx.apply();
        let ctx = ui.ctx().clone();

        self.poll_events(&ctx);

        self.spinner_angle += ctx.input(|i| i.stable_dt) * 2.8;

        if let Stage::Downloading {
            done_bytes,
            total_bytes,
        } = &self.stage
        {
            let target = if *total_bytes > 0 {
                *done_bytes as f32 / *total_bytes as f32
            } else {
                0.0
            };
            let dt = ctx.input(|i| i.stable_dt);
            self.display_progress += (target - self.display_progress) * (dt * 12.0).min(1.0);
        }

        if let Stage::Done { started } = &self.stage {
            let elapsed = started.elapsed().as_secs_f32();
            if elapsed >= DONE_COUNTDOWN {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        match &self.stage {
            Stage::WaitingForParent | Stage::Verifying | Stage::Applying => {
                ctx.request_repaint_after(std::time::Duration::from_millis(16))
            }
            Stage::Downloading { .. } => {
                ctx.request_repaint_after(std::time::Duration::from_millis(32))
            }
            _ => {}
        }

        let root_frame = Frame::NONE
            .fill(C_PRIMARY_BG)
            .stroke(Stroke::new(1.0, BORDER));

        egui::CentralPanel::default()
            .frame(root_frame)
            .show_inside(ui, |ui| {
                let do_close = draw_title_bar(ui, &self.app_icon, &self.close_icon);
                if do_close {
                    self.cancel.store(true, Ordering::Relaxed);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }

                let sep_y = ui.cursor().min.y;
                let sep_r = Rect::from_min_size(Pos2::new(0.0, sep_y), Vec2::new(WIN_W, 1.0));
                ui.painter().rect_filled(sep_r, 0.0, BORDER);
                ui.add_space(1.0);

                Frame::NONE
                    .inner_margin(Margin::symmetric(28, 0))
                    .show(ui, |ui| {
                        draw_version_banner(ui, &self.args.current_version, &self.args.new_version);

                        ui.add_space(12.0);

                        let mut cancel_clicked = false;
                        let mut retry_clicked = false;

                        draw_stage(
                            ui,
                            &self.stage,
                            self.display_progress,
                            self.spinner_angle,
                            &mut cancel_clicked,
                            &mut retry_clicked,
                        );

                        if cancel_clicked {
                            self.cancel.store(true, Ordering::Relaxed);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if retry_clicked {
                            let (cmd_tx, event_rx, cancel) = worker::spawn(self.args.clone());
                            self.cmd_tx = cmd_tx;
                            self.event_rx = event_rx;
                            self.cancel = cancel;
                            self.stage = Stage::WaitingForParent;
                            self.display_progress = 0.0;
                        }
                    });
            });
    }

    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

fn draw_title_bar(
    ui: &mut egui::Ui,
    app_icon: &Option<egui::TextureHandle>,
    close_icon: &Option<egui::TextureHandle>,
) -> bool {
    let mut closed = false;

    ui.horizontal(|ui| {
        ui.set_height(TITLE_H);

        ui.add_space(10.0);
        if let Some(icon) = app_icon {
            ui.add(egui::Image::from_texture(icon).fit_to_exact_size(Vec2::splat(18.0)));
            ui.add_space(8.0);
        }

        ui.style_mut().interaction.selectable_labels = false;
        ui.label(
            RichText::new("Nexa")
                .size(13.0)
                .color(C_TEXT_HEADER)
                .strong(),
        );
        ui.add_space(4.0);
        ui.label(RichText::new("·").size(11.0).color(C_TEXT_MUTED));
        ui.add_space(4.0);
        ui.label(RichText::new("Updater").size(12.0).color(C_TEXT_MUTED));

        let drag_rect = ui.available_rect_before_wrap();
        let drag_resp = ui.interact(drag_rect, ui.id().with("drag"), Sense::drag());
        if drag_resp.dragged_by(PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let h = ui.available_height();
            let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(BTN_W, h), Sense::click());

            if resp.hovered() {
                ui.painter().rect_filled(rect, 0.0, HOVER_CLOSE);
                resp = resp.on_hover_cursor(CursorIcon::PointingHand);
            }

            if let Some(icon) = close_icon {
                let tint = if resp.hovered() {
                    Color32::WHITE
                } else {
                    ICON_IDLE
                };
                let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON_SZ));
                ui.painter().image(
                    icon.id(),
                    icon_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    tint,
                );
            } else {
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "×",
                    FontId::proportional(16.0),
                    if resp.hovered() {
                        Color32::WHITE
                    } else {
                        ICON_IDLE
                    },
                );
            }

            if resp.clicked() {
                closed = true;
            }
        });
    });

    closed
}

fn draw_version_banner(ui: &mut egui::Ui, current: &str, next: &str) {
    ui.add_space(16.0);
    ui.style_mut().interaction.selectable_labels = false;

    ui.horizontal(|ui| {
        let avail = ui.available_width();
        let center_x = ui.cursor().min.x + avail / 2.0;

        let old_galley = ui.fonts_mut(|f| {
            f.layout_no_wrap(
                format!("v{current}"),
                FontId::proportional(12.5),
                C_TEXT_MUTED,
            )
        });

        let arrow_galley = ui.fonts_mut(|f| {
            f.layout_no_wrap("──►".to_owned(), FontId::proportional(11.0), C_BLURPLE)
        });

        let new_galley = ui.fonts_mut(|f| {
            f.layout_no_wrap(
                format!("v{next}"),
                FontId::proportional(13.5),
                C_TEXT_HEADER,
            )
        });

        let old_width = old_galley.rect.width();
        let arrow_width = arrow_galley.rect.width();
        let new_width = new_galley.rect.width();

        let total_w = old_width + 12.0 + arrow_width + 12.0 + new_width;
        let mut x = center_x - total_w / 2.0;
        let y = ui.cursor().min.y;

        draw_version_badge(ui, x, y, &old_galley, C_TEXT_MUTED, C_INPUT_BG);
        x += old_width + 16.0;

        ui.painter()
            .galley(Pos2::new(x, y + 2.0), arrow_galley, C_BLURPLE);

        x += arrow_width + 8.0;

        draw_version_badge(
            ui,
            x,
            y,
            &new_galley,
            C_TEXT_HEADER,
            C_BLURPLE.linear_multiply(0.18),
        );

        ui.allocate_space(Vec2::new(avail, old_galley.rect.height() + 12.0));
    });
}

fn draw_version_badge(
    ui: &mut egui::Ui,
    x: f32,
    y: f32,
    galley: &egui::Galley,
    text_c: Color32,
    bg_c: Color32,
) {
    let pad = Vec2::new(10.0, 5.0);
    let bg_rect = Rect::from_min_size(
        Pos2::new(x - pad.x, y - pad.y / 2.0),
        Vec2::new(
            galley.rect.width() + pad.x * 2.0,
            galley.rect.height() + pad.y,
        ),
    );
    ui.painter().rect_filled(bg_rect, 6.0, bg_c);
    ui.painter()
        .rect_stroke(bg_rect, 6.0, Stroke::new(1.0, BORDER), StrokeKind::Outside);
    ui.painter()
        .galley(Pos2::new(x, y), Arc::from(galley.clone()), text_c);
}

fn draw_stage(
    ui: &mut egui::Ui,
    stage: &Stage,
    display_prog: f32,
    spinner_angle: f32,
    cancel_out: &mut bool,
    retry_out: &mut bool,
) {
    ui.style_mut().interaction.selectable_labels = false;

    match stage {
        Stage::WaitingForParent => {
            draw_spinner_row(
                ui,
                spinner_angle,
                "Waiting for Nexa to close…",
                C_TEXT_MUTED,
            );
            ui.add_space(16.0);
            draw_sub_text(ui, "Please wait a moment.", C_TEXT_MUTED);
        }

        Stage::Downloading {
            done_bytes,
            total_bytes,
        } => {
            draw_primary_text(ui, "Downloading update…", C_TEXT_HEADER);
            ui.add_space(14.0);
            draw_progress_bar(ui, display_prog);
            ui.add_space(8.0);

            let pct = (display_prog * 100.0) as u32;
            let bytes_text = if *total_bytes > 0 {
                format!(
                    "{}%  ·  {}  /  {}",
                    pct,
                    fmt_bytes(*done_bytes),
                    fmt_bytes(*total_bytes)
                )
            } else {
                format!("{} downloaded", fmt_bytes(*done_bytes))
            };
            draw_sub_text(ui, &bytes_text, C_TEXT_MUTED);
            ui.add_space(16.0);
            draw_action_row(ui, Some("Cancel"), None, cancel_out, retry_out);
        }

        Stage::Verifying => {
            draw_spinner_row(ui, spinner_angle, "Verifying signature…", C_TEXT);
            ui.add_space(16.0);
            draw_sub_text(
                ui,
                "Checking Ed25519 signature against the embedded public key.",
                C_TEXT_MUTED,
            );
        }

        Stage::Applying => {
            draw_spinner_row(ui, spinner_angle, "Installing update…", C_TEXT);
            ui.add_space(16.0);
            draw_sub_text(ui, "Almost done — replacing the executable.", C_TEXT_MUTED);
        }

        Stage::Done { started } => {
            let remaining = (DONE_COUNTDOWN - started.elapsed().as_secs_f32())
                .max(0.0)
                .ceil() as u32;
            draw_check_row(ui, "Update complete!");
            ui.add_space(14.0);
            draw_sub_text(ui, &format!("Launching Nexa in {remaining}…"), C_TEXT_MUTED);

            let countdown_prog = 1.0 - started.elapsed().as_secs_f32() / DONE_COUNTDOWN;
            draw_countdown_bar(ui, countdown_prog.clamp(0.0, 1.0), SUCCESS);
        }

        Stage::Error { message } => {
            draw_error_row(ui, message);
            ui.add_space(16.0);
            draw_action_row(ui, Some("Cancel"), Some("Retry"), cancel_out, retry_out);
        }
    }
}

fn draw_spinner_row(ui: &mut egui::Ui, angle: f32, text: &str, color: Color32) {
    let row_h = 32.0;
    let avail = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail, row_h), Sense::hover());

    let r = 9.0_f32;
    let cx = rect.center().x - avail / 2.0 + r + 2.0;
    let cy = rect.center().y;

    let segments = 12usize;
    for i in 0..segments {
        let theta = angle + i as f32 * std::f32::consts::TAU / segments as f32;
        let dot_x = cx + r * theta.cos();
        let dot_y = cy + r * theta.sin();
        let alpha = (i as f32 / segments as f32 * 255.0) as u8;
        let c = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
        ui.painter().circle_filled(Pos2::new(dot_x, dot_y), 2.0, c);
    }

    ui.painter().text(
        Pos2::new(cx + r + 14.0, cy),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.5),
        color,
    );
}

fn draw_check_row(ui: &mut egui::Ui, text: &str) {
    let row_h = 36.0;
    let avail = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail, row_h), Sense::hover());
    let center = rect.center();

    ui.painter().circle_filled(
        Pos2::new(center.x - avail / 2.0 + 16.0, center.y),
        14.0,
        SUCCESS.linear_multiply(0.20),
    );
    ui.painter().text(
        Pos2::new(center.x - avail / 2.0 + 16.0, center.y),
        Align2::CENTER_CENTER,
        "✓",
        FontId::proportional(16.0),
        SUCCESS,
    );
    ui.painter().text(
        Pos2::new(center.x - avail / 2.0 + 38.0, center.y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(14.0),
        C_TEXT_HEADER,
    );
}

fn draw_error_row(ui: &mut egui::Ui, message: &str) {
    let avail = ui.available_width();

    let (hdr_rect, _) = ui.allocate_exact_size(Vec2::new(avail, 32.0), Sense::hover());
    let cy = hdr_rect.center().y;
    let lx = hdr_rect.min.x;

    ui.painter()
        .circle_filled(Pos2::new(lx + 14.0, cy), 13.0, DANGER.linear_multiply(0.18));
    ui.painter().text(
        Pos2::new(lx + 14.0, cy),
        Align2::CENTER_CENTER,
        "✕",
        FontId::proportional(14.0),
        DANGER,
    );
    ui.painter().text(
        Pos2::new(lx + 34.0, cy),
        Align2::LEFT_CENTER,
        "Update failed",
        FontId::proportional(13.5),
        DANGER,
    );

    ui.add_space(8.0);
    Frame::NONE
        .fill(DANGER.linear_multiply(0.08))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(avail - 0.0);
            ui.add(
                egui::Label::new(
                    RichText::new(message)
                        .size(11.0)
                        .color(DANGER.linear_multiply(1.4)),
                )
                .wrap(),
            );
        });
}

fn draw_primary_text(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).size(14.0).color(color));
}

fn draw_sub_text(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.add(egui::Label::new(RichText::new(text).size(11.0).color(color)).wrap());
}

fn draw_progress_bar(ui: &mut egui::Ui, progress: f32) {
    let avail = ui.available_width();
    let (bar_bg, _) = ui.allocate_exact_size(Vec2::new(avail, 8.0), Sense::hover());

    ui.painter().rect_filled(bar_bg, 4.0, C_INPUT_BG);
    if progress > 0.0 {
        let fill_w = (avail * progress).max(8.0);
        let fill = Rect::from_min_size(bar_bg.min, Vec2::new(fill_w, 8.0));
        ui.painter().rect_filled(fill, 4.0, C_BLURPLE);
    }
}

fn draw_countdown_bar(ui: &mut egui::Ui, progress: f32, color: Color32) {
    let avail = ui.available_width();
    let (bar_bg, _) = ui.allocate_exact_size(Vec2::new(avail, 3.0), Sense::hover());
    ui.painter().rect_filled(bar_bg, 1.5, C_INPUT_BG);
    if progress > 0.0 {
        let fill_w = (avail * progress).max(4.0);
        let fill = Rect::from_min_size(bar_bg.min, Vec2::new(fill_w, 3.0));
        ui.painter().rect_filled(fill, 1.5, color);
    }
}

fn draw_action_row(
    ui: &mut egui::Ui,
    cancel_lbl: Option<&str>,
    retry_lbl: Option<&str>,
    cancel_out: &mut bool,
    retry_out: &mut bool,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if let Some(lbl) = retry_lbl {
            if small_action_btn(ui, lbl, C_BLURPLE) {
                *retry_out = true;
            }
            ui.add_space(8.0);
        }

        if let Some(lbl) = cancel_lbl {
            if small_action_btn(ui, lbl, C_SECONDARY_BG) {
                *cancel_out = true;
            }
        }
    });
}

fn small_action_btn(ui: &mut egui::Ui, label: &str, fill: Color32) -> bool {
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), FontId::proportional(12.0), C_TEXT_HEADER)
    });
    let w = galley.rect.width() + 24.0;
    let h = galley.rect.height() + 10.0;
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());

    if resp.hovered() {
        resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let bg = if resp.is_pointer_button_down_on() {
            fill.linear_multiply(0.7)
        } else if resp.hovered() {
            fill.linear_multiply(0.85)
        } else {
            fill
        };
        ui.painter().rect_filled(rect, 6.0, bg);
        ui.painter()
            .galley(rect.min + Vec2::new(12.0, 5.0), galley, C_TEXT_HEADER);
    }
    resp.clicked()
}

fn setup_fonts(ctx: &egui::Context) {
    use eframe::epaint::{text::FontData, text::FontDefinitions, FontFamily};

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter".to_owned(),
        std::sync::Arc::from(FontData::from_static(include_bytes!(
            "../../../nexa/assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "inter".to_owned());
    ctx.set_fonts(fonts);
}

fn setup_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = Vec2::new(0.0, 0.0);
    style.spacing.window_margin = egui::Margin::same(0);
    style.visuals.window_fill = C_SECONDARY_BG;
    style.visuals.panel_fill = C_SECONDARY_BG;
    style.visuals.window_shadow = egui::Shadow::NONE;
    ctx.set_global_style(style);
}

fn fmt_bytes(b: u64) -> String {
    const MB: u64 = 1_048_576;
    const KB: u64 = 1_024;
    if b >= MB {
        format!("{:.1} MB", b as f64 / MB as f64)
    } else if b >= KB {
        format!("{:.0} KB", b as f64 / KB as f64)
    } else {
        format!("{b} B")
    }
}
