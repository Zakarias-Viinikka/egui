use std::sync::mpsc::{channel, Receiver, TryRecvError};

use eframe::egui;

use notes_core::Message;
use twitch_ban::{ban_user_permanently, BanError};

const USER_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 140, 255);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 230, 230);
const BUTTON_COL_WIDTH: f32 = 28.0;
const BUTTON_SIZE: f32 = 20.0;

enum BanStatus {
    InFlight,
    Failed(String),
    Success,
}

enum Action {
    Ban,
    Cancel,
    Close,
}

struct Viewer {
    messages: Vec<Message>,
    confirm_target: Option<usize>,
    ban_status: Option<BanStatus>,
    ban_rx: Option<Receiver<Result<(), BanError>>>,
}

impl eframe::App for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        let maybe = self.ban_rx.as_ref().map(|rx| rx.try_recv());
        match maybe {
            Some(Ok(Ok(()))) => {
                self.ban_rx = None;
                self.ban_status = Some(BanStatus::Success);
            }
            Some(Ok(Err(e))) => {
                self.ban_rx = None;
                self.ban_status = Some(BanStatus::Failed(e.to_string()));
            }
            Some(Err(TryRecvError::Disconnected)) => {
                self.ban_rx = None;
                self.ban_status =
                    Some(BanStatus::Failed("ban thread ended without a result".into()));
            }
            Some(Err(TryRecvError::Empty)) | None => {}
        }

        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let in_flight = matches!(self.ban_status, Some(BanStatus::InFlight));
        if esc && !in_flight && self.confirm_target.is_some() {
            self.confirm_target = None;
            self.ban_status = None;
        }

        if self.ban_rx.is_some() {
            ctx.request_repaint();
        }

        let mut click_ban: Option<usize> = None;

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (idx, m) in self.messages.iter().enumerate() {
                        let full_w = ui.available_width();
                        let text_w = (full_w - BUTTON_COL_WIDTH).max(0.0);
                        let row_top = ui.cursor().min;

                        ui.scope(|ui| {
                            ui.set_max_width(text_w);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(&m.user)
                                        .color(USER_COLOR)
                                        .size(18.0),
                                );
                                ui.label(
                                    egui::RichText::new(&m.text)
                                        .color(TEXT_COLOR)
                                        .size(18.0),
                                );
                            });
                        });

                        let row_bottom = ui.cursor().min.y;
                        let row_height = (row_bottom - row_top.y).max(18.0);
                        let row_rect = egui::Rect::from_min_size(
                            row_top,
                            egui::vec2(full_w, row_height),
                        );

                        let row_hovered = ui.rect_contains_pointer(row_rect);

                        let btn_center = egui::pos2(
                            row_rect.right() - BUTTON_COL_WIDTH / 2.0,
                            row_rect.center().y,
                        );
                        let btn_rect = egui::Rect::from_center_size(
                            btn_center,
                            egui::vec2(BUTTON_SIZE, BUTTON_SIZE),
                        );
                        let btn_resp = ui.interact(
                            btn_rect,
                            egui::Id::new(("ban_btn", idx)),
                            egui::Sense::click(),
                        );

                        if row_hovered || btn_resp.hovered() {
                            let bg = if btn_resp.hovered() {
                                egui::Color32::from_gray(60)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(btn_rect, 4.0, bg);
                            ui.painter().text(
                                btn_center,
                                egui::Align2::CENTER_CENTER,
                                "\u{2715}",
                                egui::FontId::proportional(14.0),
                                egui::Color32::from_gray(220),
                            );
                        }

                        if btn_resp.clicked() {
                            click_ban = Some(idx);
                        }
                    }
                });
        });

        if let Some(idx) = click_ban {
            self.confirm_target = Some(idx);
            self.ban_status = None;
        }

        if let Some(target_idx) = self.confirm_target {
            let pair = self
                .messages
                .get(target_idx)
                .map(|m| (m.user.clone(), m.user_id.clone()));
            if let Some((username, user_id)) = pair {
                let mut action: Option<Action> = None;
                egui::Modal::new(egui::Id::new("ban_confirm_modal")).show(
                    &ctx,
                    |ui| {
                        ui.set_min_width(300.0);
                        match &self.ban_status {
                            Some(BanStatus::Success) => {
                                ui.heading("Banned");
                                ui.separator();
                                ui.label(format!("{username} has been banned."));
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Close").clicked() {
                                        action = Some(Action::Close);
                                    }
                                });
                            }
                            Some(BanStatus::InFlight) => {
                                ui.heading(format!("Ban {username}?"));
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Banning\u{2026}");
                                });
                            }
                            Some(BanStatus::Failed(err)) => {
                                ui.heading(format!("Ban {username}?"));
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(err)
                                        .color(egui::Color32::from_rgb(235, 110, 110)),
                                );
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Retry").clicked() {
                                        action = Some(Action::Ban);
                                    }
                                    if ui.button("Cancel").clicked() {
                                        action = Some(Action::Cancel);
                                    }
                                });
                            }
                            None => {
                                ui.heading(format!("Ban {username}?"));
                                ui.separator();
                                ui.horizontal(|ui| {
                                    if ui.button("Confirm").clicked() {
                                        action = Some(Action::Ban);
                                    }
                                    if ui.button("Cancel").clicked() {
                                        action = Some(Action::Cancel);
                                    }
                                });
                            }
                        }
                    },
                );
                match action {
                    Some(Action::Ban) => {
                        let (tx, rx) = channel();
                        self.ban_rx = Some(rx);
                        self.ban_status = Some(BanStatus::InFlight);
                        std::thread::spawn(move || {
                            let result = ban_user_permanently(&user_id);
                            let _ = tx.send(result);
                        });
                    }
                    Some(Action::Cancel) | Some(Action::Close) => {
                        self.confirm_target = None;
                        self.ban_status = None;
                        self.ban_rx = None;
                    }
                    None => {}
                }
            } else {
                self.confirm_target = None;
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("history_viewer: expected a JSON file path argument");
            std::process::exit(1);
        }
    };
    let data = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("history_viewer: read {path} failed: {e}");
            std::process::exit(1);
        }
    };
    let messages: Vec<Message> = match serde_json::from_str(&data) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("history_viewer: parse failed: {e}");
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "chat history",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(Viewer {
                messages,
                confirm_target: None,
                ban_status: None,
                ban_rx: None,
            }))
        }),
    )
}
