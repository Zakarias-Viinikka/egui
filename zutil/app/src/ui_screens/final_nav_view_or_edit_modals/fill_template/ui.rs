use eframe::egui;
use json_parsing::template_markers::{extract_markers, fill_markers};
use std::collections::HashMap;

pub struct FillTemplateState {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub instructions: String,
    pub example: String,
    pub values: HashMap<String, String>,
    pub result: Option<String>,
    pub focused_index: Option<usize>,
    pub pending_focus: Option<usize>,
}

impl Default for FillTemplateState {
    fn default() -> Self {
        Self {
            id: 0,
            title: String::new(),
            content: String::new(),
            instructions: String::new(),
            example: String::new(),
            values: HashMap::new(),
            result: None,
            focused_index: None,
            pending_focus: None,
        }
    }
}

impl FillTemplateState {
    pub fn reset_values_for_markers(&mut self) {
        let markers = extract_markers(&self.content);
        let mut new_values = HashMap::new();
        for m in markers {
            let v = self.values.get(&m).cloned().unwrap_or_default();
            new_values.insert(m, v);
        }
        self.values = new_values;
        self.focused_index = None;
        self.pending_focus = None;
    }
}

pub enum FillTemplateAction {
    None,
    Back,
}

pub fn fill_template_ui(
    ui: &mut egui::Ui,
    state: &mut FillTemplateState,
) -> FillTemplateAction {
    let mut action = FillTemplateAction::None;

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            action = FillTemplateAction::Back;
        }
        ui.separator();
        ui.heading(&state.title);
    });

    ui.separator();

    if !state.instructions.is_empty() {
        ui.label("Instructions:");
        ui.add(egui::Label::new(&state.instructions).selectable(true));
        ui.add_space(8.0);
    }
    if !state.example.is_empty() {
        ui.label("Example:");
        ui.add(egui::Label::new(&state.example).selectable(true));
        ui.add_space(8.0);
    }

    let markers = extract_markers(&state.content);

    if !markers.is_empty() {
        let mut dir = 0i32;
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                dir = 1;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                dir = -1;
            }
        });

        if state.focused_index.is_none() {
            state.focused_index = Some(0);
            state.pending_focus = Some(0);
        }

        if dir != 0 {
            let len = markers.len();
            let cur = state.focused_index.unwrap_or(0);
            let next = if dir > 0 {
                (cur + 1) % len
            } else {
                (cur + len - 1) % len
            };
            state.focused_index = Some(next);
            state.pending_focus = Some(next);
        }
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        let width = ui.available_width();
        if ui.add_sized([width, 40.0], egui::Button::new("Generate")).clicked() {
            state.result = Some(fill_markers(&state.content, &state.values));
        }
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if markers.is_empty() {
                ui.label("no markers in this template");
            } else {
                let mut new_focus: Option<usize> = None;
                for (idx, m) in markers.iter().enumerate() {
                    ui.label(egui::RichText::new(m.as_str()).strong());
                    let response = {
                        let entry = state.values.entry(m.clone()).or_default();
                        ui.scope(|ui| {
                            let v = ui.visuals_mut();
                            v.extreme_bg_color = design::colors::FIELD_BG;
                            v.override_text_color = Some(design::colors::FIELD_TEXT);
                            v.widgets.inactive.bg_stroke =
                                egui::Stroke::new(1.0, design::colors::FIELD_BORDER);
                            v.widgets.hovered.bg_stroke =
                                egui::Stroke::new(1.5, design::colors::FIELD_BORDER_HOVER);
                            v.widgets.active.bg_stroke =
                                egui::Stroke::new(1.5, design::colors::FIELD_BORDER_HOVER);
                            ui.text_edit_singleline(entry)
                        })
                        .inner
                    };

                    if state.pending_focus == Some(idx) {
                        response.request_focus();
                    }
                    if response.has_focus() {
                        new_focus = Some(idx);
                    }
                    ui.add_space(6.0);
                }

                if let Some(i) = new_focus {
                    state.focused_index = Some(i);
                }
                if state.pending_focus.is_some() {
                    state.pending_focus = None;
                }
            }

            if let Some(result) = &state.result {
                ui.add_space(16.0);
                ui.separator();
                ui.label("Result:");
                let mut display = result.clone();
                ui.add(
                    egui::TextEdit::multiline(&mut display)
                        .desired_width(f32::INFINITY)
                        .desired_rows(10),
                );
                if ui.button("Copy result").clicked() {
                    ui.ctx().copy_text(result.clone());
                }
            }
        });
    });

    action
}
