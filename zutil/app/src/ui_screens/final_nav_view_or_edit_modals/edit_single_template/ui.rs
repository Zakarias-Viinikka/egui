use crate::components::text_input::ui::TextInput;
use crate::components::category_picker_popup::ui::{CategoryPickerState, category_picker_trigger};
use eframe::egui;

pub struct EditSingleTemplateState {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub instructions: String,
    pub example: String,
    pub category_picker: CategoryPickerState,
    pub meta_category_picker: CategoryPickerState,
}

impl Default for EditSingleTemplateState {
    fn default() -> Self {
        Self {
            id: 0,
            title: String::new(),
            content: String::new(),
            instructions: String::new(),
            example: String::new(),
            category_picker: CategoryPickerState::default(),
            meta_category_picker: CategoryPickerState::default(),
        }
    }
}

pub enum EditSingleTemplateAction {
    None,
    Back,
    Save,
}

pub fn edit_single_template_ui(
    ui: &mut egui::Ui,
    state: &mut EditSingleTemplateState,
    existing_normal_categories: &[String],
    existing_meta_categories: &[String],
) -> EditSingleTemplateAction {
    let mut action = EditSingleTemplateAction::None;

    ui.heading("Edit Template");
    ui.separator();

    ui.label("Title");
    TextInput::single("edit_template_title").show(ui, &mut state.title);

    ui.label("Content (use %%name%% for markers)");
    TextInput::multi("edit_template_content", 250.0).show(ui, &mut state.content);

    ui.label("Instructions (optional)");
    TextInput::multi("edit_template_instructions", 150.0).show(ui, &mut state.instructions);

    ui.label("Example (optional)");
    TextInput::multi("edit_template_example", 150.0).show(ui, &mut state.example);

    ui.label("Category (optional)");
    let _ = category_picker_trigger(ui, &mut state.category_picker, existing_normal_categories);

    ui.label("Meta category (optional)");
    let _ = category_picker_trigger(ui, &mut state.meta_category_picker, existing_meta_categories);

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            action = EditSingleTemplateAction::Save;
        }
        if ui.button("Back").clicked() {
            action = EditSingleTemplateAction::Back;
        }
    });

    action
}
