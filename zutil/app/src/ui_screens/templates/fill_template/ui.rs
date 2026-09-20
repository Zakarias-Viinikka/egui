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
                for m in &markers {
                    ui.label(m);
                    let entry = state.values.entry(m.clone()).or_default();
                    ui.text_edit_singleline(entry);
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
