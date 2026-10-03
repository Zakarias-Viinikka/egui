use app_core::command_router::{Line, LineStyle, Outcome, Session};
use eframe::egui;
use std::path::PathBuf;

const WINDOW_BG: egui::Color32 = egui::Color32::from_rgb(16, 16, 20);
const PROMPT: egui::Color32 = egui::Color32::from_rgb(140, 200, 140);
const TEXT: egui::Color32 = egui::Color32::from_rgb(220, 220, 230);
const ERROR: egui::Color32 = egui::Color32::from_rgb(220, 120, 120);
const FOLDER: egui::Color32 = egui::Color32::from_rgb(220, 200, 130);
const PROMPT_HISTORY: egui::Color32 = egui::Color32::from_rgb(100, 150, 100);
const ZAP: egui::Color32 = egui::Color32::from_rgb(120, 180, 220);
const CLI: egui::Color32 = egui::Color32::from_rgb(150, 150, 160);
const CURSOR: egui::Color32 = egui::Color32::from_rgb(220, 220, 230);

const PAD_X: f32 = 22.0;
const PAD_Y: f32 = 20.0;
const FONT_SIZE: f32 = 14.0;
const LINE_H: f32 = 20.0;
const CURSOR_W: f32 = 8.0;
const CURSOR_H: f32 = 16.0;
const BLINK_HZ: f64 = 1.4;

pub struct NecessaryStructForEgui {
    session: Session,
    history: Vec<Line>,
    typed: String,
    exit_requested: bool,
    /// Set in raw_input_hook, consumed at the top of ui. raw_input_hook runs
    /// before egui processes events, so it's the only place we can pull
    /// Ctrl+C out before egui's own copy handler grabs it.
    pending_action: Option<app_core::shortcuts::Action>,
}

impl Default for NecessaryStructForEgui {
    fn default() -> Self {
        Self {
            session: Session::default(),
            history: Vec::new(),
            typed: String::new(),
            exit_requested: false,
            pending_action: None,
        }
    }
}

impl eframe::App for NecessaryStructForEgui {
    fn raw_input_hook(
        &mut self,
        _ctx: &egui::Context,
        raw_input: &mut egui::RawInput,
    ) {
        // Filter Ctrl+C and Ctrl+Shift+C out of the raw events before egui
        // sees them. egui's built-in Ctrl+C copies any active text selection,
        // which fights with our own handling and clobbers the clipboard.
        // egui-winit sometimes passes Ctrl+C through as a Key event and
        // sometimes converts it to Event::Copy, depending on whether anything
        // has focus. Modifiers live on the Key event, not on RawInput, and
        // Event::Copy carries no modifiers at all. So: first pass, look for a
        // Key(C) with ctrl to read the modifiers and decide the action.
        // Second pass, strip both the Key event and any Event::Copy so egui's
        // own copy handler never fires and clobbers the clipboard.
        let mut decided: Option<app_core::shortcuts::Action> = None;

        for event in &raw_input.events {
            if let egui::Event::Key {
                key: egui::Key::C,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                if modifiers.ctrl {
                    decided = Some(if modifiers.shift {
                        app_core::shortcuts::Action::CopyAll
                    } else {
                        app_core::shortcuts::Action::ClearInput
                    });
                    break;
                }
            }
        }

        if decided.is_none()
            && raw_input.events.iter().any(|e| matches!(e, egui::Event::Copy))
        {
            // Only Event::Copy arrived. No modifiers to read, so this is
            // indistinguishable from the plain path. Fall back to ClearInput,
            // which is what plain Ctrl+C should do anyway. Ctrl+Shift+C users
            // will still get the CopyAll behaviour whenever egui-winit does
            // pass the Key event through.
            decided = Some(app_core::shortcuts::Action::ClearInput);
        }

        if decided.is_some() {
            raw_input.events.retain(|event| {
                !matches!(event, egui::Event::Copy)
                    && !matches!(
                        event,
                        egui::Event::Key {
                            key: egui::Key::C,
                            pressed: true,
                            modifiers,
                            ..
                        } if modifiers.ctrl
                    )
            });
            self.pending_action = decided;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(action) = self.pending_action.take() {
            match action {
                app_core::shortcuts::Action::ClearInput => {
                    self.typed.clear();
                }
                app_core::shortcuts::Action::CopyAll => {
                    let text = self
                        .history
                        .iter()
                        .map(|l| l.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.ctx().copy_text(text);
                    let n = self.history.len();
                    self.history.push(Line::cli(format!("copied {n} lines")));
                }
            }
        }

        let events = ui.input(|i| i.events.clone());
        for event in &events {
            match event {
                egui::Event::Text(t) => self.typed.push_str(t),
                // Paste (Ctrl+V, Ctrl+Shift+V, middle-click, whatever the WM
                // sends) arrives as Event::Paste with the whole string. It's
                // not delivered as individual Text events, so without this
                // arm nothing pastes into the custom input line.
                egui::Event::Paste(t) => self.typed.push_str(t),
                egui::Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    ..
                } => {
                    self.typed.pop();
                }
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    ..
                } => {
                    self.submit(ui);
                }
                _ => {}
            }
        }

        if self.exit_requested {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: WINDOW_BG,
                inner_margin: egui::Margin {
                    left: PAD_X as i8,
                    right: PAD_X as i8,
                    top: PAD_Y as i8,
                    bottom: PAD_Y as i8,
                },
                ..Default::default()
            })
            .show(ui, |ui| {
                self.draw(ui);
            });
    }
}

impl NecessaryStructForEgui {
    fn submit(&mut self, ui: &mut egui::Ui) {
        let typed = std::mem::take(&mut self.typed);
        let prompt = self.prompt();
        self.history.push(Line::prompt(format!("{prompt} > {typed}")));

        match app_core::command_router::route(&typed, &mut self.session) {
            Outcome::Print(lines) => self.history.extend(lines),
            Outcome::Silent => {}
            Outcome::Unknown => {}
            Outcome::Exit => {
                self.exit_requested = true;
            }
            Outcome::Clear => self.history.clear(),
            Outcome::PrintAndCopy { lines, text } => {
                self.history.extend(lines);
                ui.ctx().copy_text(text);
            }
            Outcome::CopyAll => {
                self.history.pop();
                let text = self
                    .history
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                // egui routes OutputCommand::CopyText through the backend's
                // clipboard integration, so there's no need for a shell-out.
                ui.ctx().copy_text(text);
                let n = self.history.len();
                self.history.push(Line::cli(format!("copied {n} lines")));
            }
        }
    }

    fn prompt(&self) -> String {
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
        if self.session.cwd == home {
            return "~".to_string();
        }
        if let Ok(rest) = self.session.cwd.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
        self.session.cwd.display().to_string()
    }

    fn draw(&self, ui: &mut egui::Ui) {
        let mono = egui::FontId::monospace(FONT_SIZE);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;

                for line in &self.history {
                    let color = match line.style {
                        LineStyle::Normal => TEXT,
                        LineStyle::Error => ERROR,
                        LineStyle::Folder => FOLDER,
                        LineStyle::Prompt => PROMPT_HISTORY,
                        LineStyle::Zap => ZAP,
                        LineStyle::CliMessage => CLI,
                    };
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&line.text)
                                .font(mono.clone())
                                .color(color),
                        )
                        .selectable(true)
                        .wrap_mode(egui::TextWrapMode::Wrap),
                    );
                }

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(
                        egui::RichText::new(format!("{} > ", self.prompt()))
                            .font(mono.clone())
                            .color(PROMPT),
                    );
                    let typed_resp = ui.label(
                        egui::RichText::new(&self.typed)
                            .font(mono.clone())
                            .color(TEXT),
                    );

                    let cursor_x = typed_resp.rect.right() + 2.0;
                    let cursor_y = typed_resp.rect.center().y;
                    let t = ui.input(|i| i.time);
                    let blink_on = ((t * BLINK_HZ) as i64) % 2 == 0;
                    if blink_on {
                        let cursor_rect = egui::Rect::from_min_size(
                            egui::pos2(cursor_x, cursor_y - CURSOR_H * 0.5),
                            egui::vec2(CURSOR_W, CURSOR_H),
                        );
                        ui.painter().rect_filled(cursor_rect, 1.0, CURSOR);
                    }
                });
            });

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(350));
    }
}
