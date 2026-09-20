use eframe::egui;

pub struct EditSingleCategoryState {
    pub id: i64,
    pub name: String,
    pub original_name: String,
}

impl Default for EditSingleCategoryState {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            original_name: String::new(),
        }
    }
}

pub enum EditSingleCategoryAction {
    None,
    Back,
    Save,
}

pub fn edit_single_category_ui(
    ui: &mut egui::Ui,
    state: &mut EditSingleCategoryState,
) -> EditSingleCategoryAction {
    let mut action = EditSingleCategoryAction::None;

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            action = EditSingleCategoryAction::Back;
        }
        ui.separator();
        ui.heading("Edit category");
    });

    ui.separator();

    ui.label(egui::RichText::new("Name").strong());
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
        ui.text_edit_singleline(&mut state.name);
    });

    ui.add_space(16.0);

    let can_save = !state.name.trim().is_empty() && state.name != state.original_name;
    if ui
        .add_enabled(can_save, egui::Button::new("Save"))
        .clicked()
    {
        action = EditSingleCategoryAction::Save;
    }

    action
}
