use crate::components::category_picker_popup::ui::{CategoryPickerState, category_picker_trigger};
use eframe::egui;
use popup::ui::{PopupAction, PopupButton, PopupParams, popup};

pub struct CreateTemplateState {
    pub title: String,
    pub content: String,
    pub instructions: String,
    pub example: String,
    pub category_picker: CategoryPickerState,
    pub meta_category_picker: CategoryPickerState,
    pub show_confirm: bool,
}

impl Default for CreateTemplateState {
    fn default() -> Self {
        Self {
            title: String::new(),
            content: String::new(),
            instructions: String::new(),
            example: String::new(),
            category_picker: CategoryPickerState::default(),
            meta_category_picker: CategoryPickerState::default(),
            show_confirm: false,
        }
    }
}

pub enum CreateTemplateAction {
    None,
    Back,
    Create,
}

pub fn create_template_ui(
    ui: &mut egui::Ui,
    state: &mut CreateTemplateState,
    existing_normal_categories: &[String],
    existing_meta_categories: &[String],
) -> CreateTemplateAction {
    let mut action = CreateTemplateAction::None;

    ui.heading("New Template");
    ui.separator();

    ui.label("Title");
    ui.text_edit_singleline(&mut state.title);

    ui.label("Content (use %%name%% for markers)");
    ui.text_edit_multiline(&mut state.content);

    ui.label("Instructions (optional)");
    ui.text_edit_multiline(&mut state.instructions);

    ui.label("Example (optional)");
    ui.text_edit_multiline(&mut state.example);

    ui.label("Category (optional)");
    let _ = category_picker_trigger(ui, &mut state.category_picker, existing_normal_categories);

    ui.label("Meta category (optional)");
    let _ = category_picker_trigger(ui, &mut state.meta_category_picker, existing_meta_categories);

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Create").clicked() {
            state.show_confirm = true;
        }
        if ui.button("Back").clicked() {
            action = CreateTemplateAction::Back;
        }
    });

    if state.show_confirm {
        let title = state.title.clone();
        let content = state.content.clone();
        let instructions = state.instructions.clone();
        let example = state.example.clone();
        let category = state.category_picker.current.clone();
        let meta = state.meta_category_picker.current.clone();
        let popup_action = popup(
            ui,
            PopupParams {
                id: "create_template_confirm",
                title: "Confirm create",
                page_label: None,
                show_nav: false,
                copy_text: None,
                buttons: vec![
                    PopupButton { label: "Confirm".to_string(), action: PopupAction::Confirm },
                    PopupButton { label: "Cancel".to_string(), action: PopupAction::Cancel },
                ],
                body: Box::new(move |ui| {
                    ui.label(format!("title: {}", title));
                    ui.label(format!("content: {}", content));
                    if !instructions.is_empty() {
                        ui.label(format!("instructions: {}", instructions));
                    }
                    if !example.is_empty() {
                        ui.label(format!("example: {}", example));
                    }
                    if !category.is_empty() {
                        ui.label(format!("category: {}", category));
                    }
                    if !meta.is_empty() {
                        ui.label(format!("meta category: {}", meta));
                    }
                }),
            },
        );

        match popup_action {
            PopupAction::Confirm => {
                state.show_confirm = false;
                action = CreateTemplateAction::Create;
            }
            PopupAction::Cancel => {
                state.show_confirm = false;
            }
            _ => {}
        }
    }

    action
}
