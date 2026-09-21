use eframe::egui;

enum Mode {
    Single,
    Multi { max_height: Option<f32> },
}

pub struct TextInput<'a> {
    id: &'a str,
    hint: &'a str,
    mode: Mode,
    read_only: bool,
    desired_rows: usize,
}

impl<'a> TextInput<'a> {
    pub fn single(id: &'a str) -> Self {
        Self {
            id,
            hint: "",
            mode: Mode::Single,
            read_only: false,
            desired_rows: 1,
        }
    }

    pub fn multi(id: &'a str, max_height: f32) -> Self {
        Self {
            id,
            hint: "",
            mode: Mode::Multi { max_height: Some(max_height) },
            read_only: false,
            desired_rows: 6,
        }
    }

    /// Multiline, no internal scroll wrapper. Use when already inside a ScrollArea.
    pub fn plain_multi(id: &'a str) -> Self {
        Self {
            id,
            hint: "",
            mode: Mode::Multi { max_height: None },
            read_only: false,
            desired_rows: 10,
        }
    }

    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = rows;
        self
    }

    pub fn show(self, ui: &mut egui::Ui, text: &mut String) -> egui::Response {
        match self.mode {
            Mode::Single => {
                let mut edit = egui::TextEdit::singleline(text)
                    .desired_width(f32::INFINITY);
                if self.read_only {
                    edit = edit.interactive(false);
                }
                if !self.hint.is_empty() {
                    edit = edit.hint_text(self.hint);
                }
                ui.add(edit)
            }
            Mode::Multi { max_height: Some(max_h) } => {
                egui::ScrollArea::vertical()
                    .id_salt(self.id)
                    .max_height(max_h)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let mut edit = egui::TextEdit::multiline(text)
                            .desired_width(f32::INFINITY)
                            .desired_rows(self.desired_rows);
                        if self.read_only {
                            edit = edit.interactive(false);
                        }
                        if !self.hint.is_empty() {
                            edit = edit.hint_text(self.hint);
                        }
                        ui.add(edit)
                    })
                    .inner
            }
            Mode::Multi { max_height: None } => {
                let mut edit = egui::TextEdit::multiline(text)
                    .desired_width(f32::INFINITY)
                    .desired_rows(self.desired_rows);
                if self.read_only {
                    edit = edit.interactive(false);
                }
                if !self.hint.is_empty() {
                    edit = edit.hint_text(self.hint);
                }
                ui.add(edit)
            }
        }
    }
}
