use crate::components::text_input::ui::TextInput;
use crate::components::category_picker_popup::ui::{CategoryPickerState, category_picker_trigger};
use eframe::egui;

pub struct EditSingleTextState {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub type_of_text: String,
    pub category_picker: CategoryPickerState,
    pub meta_category_picker: CategoryPickerState,
}

impl Default for EditSingleTextState {
    fn default() -> Self {
        Self {
            id: 0,
            title: String::new(),
            body: String::new(),
            type_of_text: String::new(),
            category_picker: CategoryPickerState::default(),
            meta_category_picker: CategoryPickerState::default(),
        }
    }
}

pub enum EditSingleTextAction {
    None,
    Back,
    Save,
}

pub fn edit_single_text_ui(
    ui: &mut egui::Ui,
    state: &mut EditSingleTextState,
    existing_normal_categories: &[String],
    existing_meta_categories: &[String],
) -> EditSingleTextAction {
    let mut action = EditSingleTextAction::None;

    ui.heading("Edit Text");
    ui.separator();

    ui.label("Title");
    TextInput::single("edit_text_title").show(ui, &mut state.title);

    ui.label("Body");
    TextInput::multi("edit_text_body", 250.0).show(ui, &mut state.body);

    ui.label("Type of text (optional)");
    TextInput::single("edit_text_type").show(ui, &mut state.type_of_text);

    ui.label("Category (optional)");
    let _ = category_picker_trigger(ui, &mut state.category_picker, existing_normal_categories);

    ui.label("Meta category (optional)");
    let _ = category_picker_trigger(ui, &mut state.meta_category_picker, existing_meta_categories);

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            action = EditSingleTextAction::Save;
        }
        if ui.button("Back").clicked() {
            action = EditSingleTextAction::Back;
        }
    });

    action
}
