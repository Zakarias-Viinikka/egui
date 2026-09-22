use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::hotkeys::Cmd;
use crate::twitch;

const FADE_AFTER: Duration = Duration::from_secs(15);
const FADE_DURATION: Duration = Duration::from_secs(2);
const VISIBLE_MAX: usize = 10;
const MAX_BOX_WIDTH: f32 = 400.0;

const USER_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 140, 255);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 230, 230);

struct Message {
    user: String,
    text: String,
    arrived: Instant,
}

pub struct App {
    rx: Option<Receiver<(String, String)>>,
    cmds: Receiver<Cmd>,
    messages: Vec<Message>,
    show_history: bool,
    hotkey_hidden: bool,
}

impl App {
    pub fn new(cmds: Receiver<Cmd>) -> Self {
        Self {
            rx: None,
            cmds,
            messages: Vec::new(),
            show_history: false,
            hotkey_hidden: false,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint();

        let mut visuals = ui.style().visuals.clone();
        visuals.window_fill = egui::Color32::TRANSPARENT;
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.popup_shadow = egui::epaint::Shadow::NONE;
        ui.ctx().set_visuals(visuals);

        if self.rx.is_none() {
            self.rx = Some(twitch::spawn_reader(ui.ctx().clone()));
        }

        if let Some(rx) = &self.rx {
            while let Ok((user, text)) = rx.try_recv() {
                self.messages.push(Message {
                    user,
                    text,
                    arrived: Instant::now(),
                });
            }
        }

        while let Ok(cmd) = self.cmds.try_recv() {
            match cmd {
                Cmd::ToggleHistory => self.show_history = !self.show_history,
                Cmd::ToggleHidden => self.hotkey_hidden = !self.hotkey_hidden,
                Cmd::Quit => std::process::exit(0),
            }
        }

        let pointer_over = ui.ctx().input(|i| i.pointer.hover_pos().is_some());
        let content_visible = !self.hotkey_hidden && !pointer_over;

        if !content_visible {
            return;
        }

        if self.show_history {
            self.draw_history(ui);
        } else {
            self.draw_live(ui);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

impl App {
    fn draw_live(&self, ui: &mut egui::Ui) {
        let now = Instant::now();

        let visible: Vec<&Message> = self
            .messages
            .iter()
            .filter(|m| now.duration_since(m.arrived) < FADE_AFTER + FADE_DURATION)
            .collect();

        let start = visible.len().saturating_sub(VISIBLE_MAX);
        let visible = &visible[start..];

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 230))
                    .shadow(egui::epaint::Shadow::NONE)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_max_width(MAX_BOX_WIDTH - 16.0);
                        for m in visible {
                            let age = now.duration_since(m.arrived);
                            let alpha = if age <= FADE_AFTER {
                                1.0
                            } else {
                                let t =
                                    (age - FADE_AFTER).as_secs_f32() / FADE_DURATION.as_secs_f32();
                                (1.0 - t).clamp(0.0, 1.0)
                            };
                            let a = (alpha * 255.0) as u8;
                            let user_color = egui::Color32::from_rgba_unmultiplied(
                                USER_COLOR.r(),
                                USER_COLOR.g(),
                                USER_COLOR.b(),
                                a,
                            );
                            let text_color = egui::Color32::from_rgba_unmultiplied(
                                TEXT_COLOR.r(),
                                TEXT_COLOR.g(),
                                TEXT_COLOR.b(),
                                a,
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(&m.user).color(user_color).size(18.0),
                                );
                                ui.label(
                                    egui::RichText::new(&m.text).color(text_color).size(18.0),
                                );
                            });
                        }
                    });
            });
    }

    fn draw_history(&self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_black_alpha(220))
                    .shadow(egui::epaint::Shadow::NONE)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_max_width(MAX_BOX_WIDTH - 16.0);
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for m in self.messages.iter() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new(&m.user).color(USER_COLOR).size(18.0),
                                    );
                                    ui.label(
                                        egui::RichText::new(&m.text).color(TEXT_COLOR).size(18.0),
                                    );
                                });
                            }
                        });
                    });
            });
    }
}
