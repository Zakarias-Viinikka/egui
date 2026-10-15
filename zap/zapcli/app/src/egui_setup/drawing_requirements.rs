use app_core::command_router::{Line, LineStyle, Outcome, Session};
use app_core::general_util::socket;
use eframe::egui;
use std::path::PathBuf;
use zap_wire::FrameKind;

const WINDOW_BG: egui::Color32 = egui::Color32::from_rgb(16, 16, 20);
const PROMPT: egui::Color32 = egui::Color32::from_rgb(140, 200, 140);
const TEXT: egui::Color32 = egui::Color32::from_rgb(220, 220, 230);
const ERROR: egui::Color32 = egui::Color32::from_rgb(220, 120, 120);
const FOLDER: egui::Color32 = egui::Color32::from_rgb(220, 200, 130);
const PROMPT_HISTORY: egui::Color32 = egui::Color32::from_rgb(100, 150, 100);
const ZAP: egui::Color32 = egui::Color32::from_rgb(120, 180, 220);
const CLI: egui::Color32 = egui::Color32::from_rgb(150, 150, 160);
const PEEK: egui::Color32 = egui::Color32::from_rgb(170, 140, 200);

const PANE_BG: egui::Color32 = egui::Color32::from_rgb(10, 10, 14);
const PANE_BORDER: egui::Color32 = egui::Color32::from_rgb(60, 60, 70);
const PANE_TITLE: egui::Color32 = egui::Color32::from_rgb(180, 200, 220);
const PANE_STATUS: egui::Color32 = egui::Color32::from_rgb(150, 150, 160);

const PAD_X: f32 = 22.0;
const PAD_Y: f32 = 20.0;
const FONT_SIZE: f32 = 14.0;

const STREAM_LINE_CAP: usize = 5000;
const STREAM_PANE_FRACTION: f32 = 0.45;

pub struct NecessaryStructForEgui {
    session: Session,
    history: Vec<Line>,
    typed: String,
    exit_requested: bool,
    submit_requested: bool,
    pending_action: Option<app_core::shortcuts::Action>,
    stream: Option<StreamState>,
}

struct StreamState {
    session: socket::Session,
    title: String,
    folder: zap_wire::linefold::LineFolder,
    done: bool,
    status: String,
}

enum PaneAction {
    None,
    Clear,
    Copy,
    Close,
}

impl Default for NecessaryStructForEgui {
    fn default() -> Self {
        Self {
            session: Session::default(),
            history: Vec::new(),
            typed: String::new(),
            exit_requested: false,
            submit_requested: false,
            pending_action: None,
            stream: None,
        }
    }
}

impl eframe::App for NecessaryStructForEgui {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let mut decided: Option<app_core::shortcuts::Action> = None;
        let has_stream = self.stream.is_some();

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
                    } else if has_stream {
                        app_core::shortcuts::Action::CancelStream
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
            decided = Some(if has_stream {
                app_core::shortcuts::Action::CancelStream
            } else {
                app_core::shortcuts::Action::ClearInput
            });
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

        // Plain Enter submits (stripped so TextEdit never sees it).
        // Shift+Enter passes through so TextEdit inserts a newline. Tab is
        // stripped so it can't move focus to a pane button and get pressed.
        let mut submit_enter = false;
        raw_input.events.retain(|event| {
            if let egui::Event::Key {
                key: egui::Key::Enter,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                if !modifiers.shift {
                    submit_enter = true;
                    return false;
                }
            }
            if let egui::Event::Key {
                key: egui::Key::Tab,
                pressed: true,
                ..
            } = event
            {
                return false;
            }
            true
        });
        if submit_enter {
            self.submit_requested = true;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_stream_frames(ui.ctx());

        if let Some(action) = self.pending_action.take() {
            match action {
                app_core::shortcuts::Action::ClearInput => {
                    self.typed.clear();
                }
                app_core::shortcuts::Action::CancelStream => {
                    if let Some(stream) = &mut self.stream {
                        let _ = stream.session.send_cancel();
                    }
                    self.stream = None;
                }
                app_core::shortcuts::Action::CopyAll => {
                    let (text, n, skipped) = self.build_scrollback(false);
                    ui.ctx().copy_text(text);
                    let msg = if skipped > 0 {
                        format!("copied {n} lines ({skipped} peek lines skipped)")
                    } else {
                        format!("copied {n} lines")
                    };
                    self.history.push(Line::cli(msg));
                }
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

        if std::mem::take(&mut self.submit_requested) {
            self.submit(ui.ctx());
            ui.ctx().request_repaint();
        }
    }
}

impl NecessaryStructForEgui {
    fn drain_stream_frames(&mut self, ctx: &egui::Context) {
        let mut notices: Vec<String> = Vec::new();
        let mut repaint = false;
        if let Some(stream) = &mut self.stream {
            loop {
                match stream.session.try_recv() {
                    Some(f) => match f.kind {
                        FrameKind::StreamOpen => stream.title = f.text,
                        FrameKind::StreamData => stream.folder.push_str(&f.text),
                        FrameKind::StreamClear => {
                            stream.folder.clear();
                        }
                        FrameKind::StreamDone => {
                            stream.folder.finish();
                            stream.done = true;
                            stream.status = f.text;
                        }
                        FrameKind::StreamClose => {
                            stream.done = true;
                        }
                        FrameKind::Notice => notices.push(f.text),
                        FrameKind::Reply => notices.push(f.text),
                        _ => {}
                    },
                    None => break,
                }
            }
            if stream.folder.lines.len() > STREAM_LINE_CAP {
                let drop = stream.folder.lines.len() - STREAM_LINE_CAP;
                stream.folder.lines.drain(0..drop);
            }
            repaint = !stream.done;
        }
        for n in notices {
            self.history.push(Line::cli(n));
        }
        if repaint {
            ctx.request_repaint();
        }
    }

    fn submit(&mut self, ctx: &egui::Context) {
        if let Some(s) = &self.stream {
            if !s.done {
                self.history
                    .push(Line::error("zapcli: a run is already in progress"));
                return;
            }
        }

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
                ctx.copy_text(text);
            }
            Outcome::CopyAll { include_peeks } => {
                self.history.pop();
                let (text, n, skipped) = self.build_scrollback(include_peeks);
                ctx.copy_text(text);
                let msg = if skipped > 0 {
                    format!("copied {n} lines ({skipped} peek lines skipped)")
                } else {
                    format!("copied {n} lines")
                };
                self.history.push(Line::cli(msg));
            }
            Outcome::Peek(lines) => {
                if let Some(last) = self.history.last_mut() {
                    last.skip_copy = true;
                }
                self.history.extend(lines);
            }
            Outcome::Streaming(sess) => {
                self.stream = Some(StreamState {
                    session: sess,
                    title: String::new(),
                    folder: zap_wire::linefold::LineFolder::default(),
                    done: false,
                    status: String::new(),
                });
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

    fn draw(&mut self, ui: &mut egui::Ui) {
        let total_h = ui.available_height();
        let pane_h = if self.stream.is_some() {
            total_h * STREAM_PANE_FRACTION
        } else {
            0.0
        };
        let top_h = total_h - pane_h;

        let width = ui.available_width();
        ui.allocate_ui(egui::vec2(width, top_h), |ui| {
            self.draw_history(ui);
        });

        if pane_h > 0.0 {
            let action = ui
                .allocate_ui(egui::vec2(width, pane_h), |ui| self.draw_pane(ui))
                .inner;
            match action {
                PaneAction::None => {}
                PaneAction::Clear => {
                    if let Some(s) = &mut self.stream {
                        s.folder.clear();
                    }
                }
                PaneAction::Copy => {
                    if let Some(s) = &self.stream {
                        let text = s.folder.lines.join("\n");
                        ui.ctx().copy_text(text);
                    }
                }
                PaneAction::Close => {
                    self.stream = None;
                }
            }
        }

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(350));
    }

    fn build_scrollback(&self, include_peeks: bool) -> (String, usize, usize) {
        let mut lines: Vec<&str> = Vec::new();
        let mut skipped = 0usize;
        for line in &self.history {
            if line.skip_copy && !include_peeks {
                skipped += 1;
                continue;
            }
            lines.push(&line.text);
        }
        let mut text = lines.join("\n");
        if let Some(stream) = &self.stream {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&stream.folder.lines.join("\n"));
        }
        (text, lines.len(), skipped)
    }

    fn draw_history(&mut self, ui: &mut egui::Ui) {
        let mono = egui::FontId::monospace(FONT_SIZE);
        let prompt = self.prompt();
        egui::ScrollArea::vertical()
            .id_salt("history")
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
                        LineStyle::Peek => PEEK,
                    };
                    let is_peek = matches!(line.style, LineStyle::Peek);
                    let frame = egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(10, 0))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&line.text)
                                        .font(mono.clone())
                                        .color(color),
                                )
                                .selectable(true)
                                .wrap_mode(egui::TextWrapMode::Wrap),
                            )
                        });
                    if is_peek {
                        let outer = frame.response.rect;
                        let inner = frame.inner.rect;
                        let bar = egui::Rect::from_min_size(
                            egui::pos2(outer.left() + 2.0, inner.top()),
                            egui::vec2(2.0, inner.height()),
                        );
                        ui.painter().rect_filled(bar, 0.0, PEEK);
                    }
                }

                let id = egui::Id::new("zapcli_input");
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(
                        egui::RichText::new(format!("{prompt} > "))
                            .font(mono.clone())
                            .color(PROMPT),
                    );
                    // Ask the horizontal layout for its real remaining
                    // width after the prompt. A guessed width made the
                    // TextEdit wrap past the right edge.
                    let remaining = ui.available_width().max(80.0);
                    let out = egui::TextEdit::multiline(&mut self.typed)
                        .id(id)
                        .font(mono.clone())
                        .text_color(TEXT)
                        .desired_rows(1)
                        .desired_width(remaining)
                        .frame(egui::Frame::NONE)
                        .show(ui);
                    // Focus the input when nothing else holds focus. This
                    // runs at most one frame per focus loss, so it can't
                    // spin the render loop like request_focus() every frame.
                    if ui.memory(|m| m.focused().is_none()) {
                        out.response.request_focus();
                    }
                });
            });
    }

    fn draw_pane(&self, ui: &mut egui::Ui) -> PaneAction {
        let Some(stream) = &self.stream else {
            return PaneAction::None;
        };
        let mut action = PaneAction::None;

        let pane_rect = ui.available_rect_before_wrap();
        ui.painter().rect(
            pane_rect,
            4.0,
            PANE_BG,
            egui::Stroke::new(1.0, PANE_BORDER),
            egui::StrokeKind::Inside,
        );

        let inner = pane_rect.shrink(8.0);

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        let ui = &mut child;

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(if stream.title.is_empty() {
                    "(stream)"
                } else {
                    &stream.title
                })
                .strong()
                .color(PANE_TITLE),
            );
            if stream.done && !stream.status.is_empty() {
                ui.label(
                    egui::RichText::new(&stream.status)
                        .italics()
                        .color(PANE_STATUS),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    action = PaneAction::Close;
                }
                if ui.button("Copy").clicked() {
                    action = PaneAction::Copy;
                }
                if ui.button("Clear").clicked() {
                    action = PaneAction::Clear;
                }
            });
        });

        ui.separator();

        let mono = egui::FontId::monospace(FONT_SIZE - 1.0);
        egui::ScrollArea::vertical()
            .id_salt("stream_pane")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                for line in &stream.folder.lines {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(line)
                                .font(mono.clone())
                                .color(TEXT),
                        )
                        .selectable(true)
                        .wrap_mode(egui::TextWrapMode::Wrap),
                    );
                }
                if !stream.folder.current.is_empty() {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&stream.folder.current)
                                .font(mono.clone())
                                .color(TEXT),
                        )
                        .selectable(true)
                        .wrap_mode(egui::TextWrapMode::Wrap),
                    );
                }
            });

        action
    }
}
