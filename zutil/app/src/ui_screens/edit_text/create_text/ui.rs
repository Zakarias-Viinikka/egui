use crate::components::category_picker_popup::ui::{CategoryPickerState, category_picker_trigger};
use eframe::egui;
use popup::ui::{PopupAction, PopupButton, PopupParams, popup};

pub struct CreateTextState {
    pub title: String,
    pub body: String,
    pub type_of_text: String,
    pub category_picker: CategoryPickerState,
    pub meta_category_picker: CategoryPickerState,
    pub show_confirm: bool,
}

impl Default for CreateTextState {
    fn default() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            type_of_text: String::new(),
            category_picker: CategoryPickerState::default(),
            meta_category_picker: CategoryPickerState::default(),
            show_confirm: false,
        }
    }
}

pub enum CreateTextAction {
    None,
    Back,
    Create,
}

pub fn create_text_ui(
    ui: &mut egui::Ui,
    state: &mut CreateTextState,
    existing_normal_categories: &[String],
    existing_meta_categories: &[String],
) -> CreateTextAction {
    let mut action = CreateTextAction::None;

    ui.heading("New Text");
    ui.separator();

    ui.label("Title");
    ui.text_edit_singleline(&mut state.title);

    ui.label("Body");
    ui.text_edit_multiline(&mut state.body);

    ui.label("Type of text (optional)");
    ui.text_edit_singleline(&mut state.type_of_text);

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
            action = CreateTextAction::Back;
        }
    });

    if state.show_confirm {
        let title = state.title.clone();
        let body = state.body.clone();
        let type_of_text = state.type_of_text.clone();
        let category = state.category_picker.current.clone();
        let meta = state.meta_category_picker.current.clone();
        let popup_action = popup(
            ui,
            PopupParams {
                id: "create_text_confirm",
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
                    ui.label(format!("body: {}", body));
                    if !type_of_text.is_empty() {
                        ui.label(format!("type: {}", type_of_text));
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
                action = CreateTextAction::Create;
            }
            PopupAction::Cancel => {
                state.show_confirm = false;
            }
            _ => {}
        }
    }

    action
}
