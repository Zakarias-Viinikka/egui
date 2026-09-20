use eframe::egui;

pub struct CategoryPickerState {
    pub show: bool,
    pub creating_new: bool,
    pub new_name: String,
    pub current: String,
}

impl Default for CategoryPickerState {
    fn default() -> Self {
        Self {
            show: false,
            creating_new: false,
            new_name: String::new(),
            current: String::new(),
        }
    }
}

pub enum CategoryPickerResult {
    None,
    Picked(String),
    Cleared,
}

pub fn category_picker_trigger(
    ui: &mut egui::Ui,
    state: &mut CategoryPickerState,
    existing: &[String],
) -> CategoryPickerResult {
    let mut result = CategoryPickerResult::None;

    let label = if state.current.is_empty() {
        "Pick category".to_string()
    } else {
        state.current.clone()
    };
    if ui.button(label).clicked() {
        state.show = true;
        state.creating_new = false;
        state.new_name.clear();
    }

    if state.show {
        egui::Modal::new(egui::Id::new("category_picker")).show(ui.ctx(), |ui| {
            ui.set_width(360.0);

            if state.creating_new {
                ui.heading("New category");
                ui.separator();
                ui.text_edit_singleline(&mut state.new_name);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Confirm").clicked() && !state.new_name.is_empty() {
                        state.current = state.new_name.clone();
                        result = CategoryPickerResult::Picked(state.new_name.clone());
                        state.show = false;
                        state.creating_new = false;
                    }
                    if ui.button("Cancel").clicked() {
                        state.creating_new = false;
                    }
                });
            } else {
                ui.heading("Pick category");
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for name in existing {
                            if ui.button(name).clicked() {
                                state.current = name.clone();
                                result = CategoryPickerResult::Picked(name.clone());
                                state.show = false;
                            }
                        }
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("New category").clicked() {
                        state.creating_new = true;
                    }
                    if !state.current.is_empty() && ui.button("Clear").clicked() {
                        state.current.clear();
                        result = CategoryPickerResult::Cleared;
                        state.show = false;
                    }
                    if ui.button("Cancel").clicked() {
                        state.show = false;
                    }
                });
            }
        });
    }

    result
}
