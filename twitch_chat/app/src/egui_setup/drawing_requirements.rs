use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::hotkeys::Cmd;
use crate::twitch;

const FADE_AFTER: Duration = Duration::from_secs(15);
const FADE_DURATION: Duration = Duration::from_secs(2);
const VISIBLE_MAX: usize = 10;
const MAX_BOX_WIDTH: f32 = 400.0;
const MAX_MESSAGES: usize = 30;

const USER_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 140, 255);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 230, 230);

struct Message {
    user: String,
    text: String,
    user_id: String,
    arrived: Instant,
}

pub struct App {
    rx: Option<Receiver<(String, String, String)>>,
    cmds: Receiver<Cmd>,
    messages: VecDeque<Message>,
    hotkey_hidden: bool,
    history_child: Option<std::process::Child>,
}

impl App {
    pub fn new(cmds: Receiver<Cmd>) -> Self {
        Self {
            rx: None,
            cmds,
            messages: VecDeque::new(),
            hotkey_hidden: false,
            history_child: None,
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
            while let Ok((user, text, user_id)) = rx.try_recv() {
                self.messages.push_back(Message {
                    user,
                    text,
                    user_id,
                    arrived: Instant::now(),
                });
                while self.messages.len() > MAX_MESSAGES {
                    self.messages.pop_front();
                }
            }
        }

        let exited = match self.history_child.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(Some(_)) | Err(_)),
            None => false,
        };
        if exited {
            self.history_child = None;
        }

        while let Ok(cmd) = self.cmds.try_recv() {
            match cmd {
                Cmd::SpawnHistory => self.toggle_history(),
                Cmd::ToggleHidden => self.hotkey_hidden = !self.hotkey_hidden,
                Cmd::Quit => std::process::exit(0),
            }
        }

        let pointer_over = ui.ctx().input(|i| i.pointer.hover_pos().is_some());
        let content_visible = !self.hotkey_hidden && !pointer_over;

        if !content_visible {
            return;
        }

        self.draw_live(ui);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

impl App {
    fn toggle_history(&mut self) {
        if let Some(mut child) = self.history_child.take() {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }

        let snapshot: Vec<notes_core::Message> = self
            .messages
            .iter()
            .map(|m| notes_core::Message {
                user: m.user.clone(),
                text: m.text.clone(),
                user_id: m.user_id.clone(),
            })
            .collect();
        let json = match serde_json::to_string(&snapshot) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("history: serialize failed: {e}");
                return;
            }
        };
        let path = std::env::temp_dir().join("twitch_chat_history.json");
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("history: write {} failed: {e}", path.display());
            return;
        }
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let viewer = match exe_dir {
            Some(d) => d.join("history_viewer"),
            None => {
                eprintln!("history: cannot determine exe dir");
                return;
            }
        };
        if !viewer.exists() {
            eprintln!("history: viewer not found at {}", viewer.display());
            return;
        }
        match std::process::Command::new(&viewer).arg(&path).spawn() {
            Ok(child) => {
                self.history_child = Some(child);
            }
            Err(e) => eprintln!("history: spawn failed: {e}"),
        }
    }

    fn draw_live(&self, ui: &mut egui::Ui) {
        let now = Instant::now();

        let visible: Vec<&Message> = self
            .messages
            .iter()
            .filter(|m| now.duration_since(m.arrived) < FADE_AFTER + FADE_DURATION)
            .collect();

        let start = visible.len().saturating_sub(VISIBLE_MAX);
        let visible = &visible[start..];

        if visible.is_empty() {
            return;
        }

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
}
