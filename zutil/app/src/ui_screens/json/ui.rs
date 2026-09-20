use eframe::egui;
use json_parsing::new_category::NewCategory;
use json_parsing::new_template::NewTemplate;
use json_parsing::new_text::NewText;
use json_parsing::parse::{ParsedPayload, parse_pasted_json};
use popup::ui::{PopupAction, PopupButton, PopupParams, popup};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum JsonMode {
    Import,
    Export,
    Validate,
}

impl JsonMode {
    pub fn has_do_button(self) -> bool {
        matches!(self, JsonMode::Import | JsonMode::Export)
    }

    pub fn do_label(self) -> &'static str {
        match self {
            JsonMode::Import => "Import",
            JsonMode::Export => "Export",
            JsonMode::Validate => "",
        }
    }

    pub fn has_prompt(self) -> bool {
        matches!(self, JsonMode::Import | JsonMode::Export)
    }
}

pub struct JsonState {
    pub mode: Option<JsonMode>,
    pub content: String,
    pub popup_payloads: Option<Vec<ParsedPayload>>,
    pub popup_error: Option<String>,
}

impl Default for JsonState {
    fn default() -> Self {
        Self {
            mode: Some(JsonMode::Import),
            content: String::new(),
            popup_payloads: None,
            popup_error: None,
        }
    }
}

pub enum JsonAction {
    None,
    Back,
    Example,
    Do,
    ImportConfirmed(Vec<ParsedPayload>),
}

pub fn json_ui(ui: &mut egui::Ui, state: &mut JsonState) -> JsonAction {
    let mut action = JsonAction::None;
    let mut try_parse = false;

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            action = JsonAction::Back;
        }
        ui.separator();
        if ui.selectable_label(state.mode == Some(JsonMode::Import), "Import").clicked() {
            state.mode = Some(JsonMode::Import);
        }
        if ui.selectable_label(state.mode == Some(JsonMode::Export), "Export").clicked() {
            state.mode = Some(JsonMode::Export);
        }
        if ui.selectable_label(state.mode == Some(JsonMode::Validate), "Validate").clicked() {
            state.mode = Some(JsonMode::Validate);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Example").clicked() {
                action = JsonAction::Example;
            }
        });
    });

    ui.separator();

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(16.0);

        let mode = state.mode;
        let has_do = mode.map(|m| m.has_do_button()).unwrap_or(false);
        let has_prompt = mode.map(|m| m.has_prompt()).unwrap_or(false);

        let total_width = ui.available_width();

        if has_do && has_prompt {
            let prompt_width = total_width / 5.0;
            let do_width = total_width - prompt_width - ui.spacing().item_spacing.x;

            ui.horizontal(|ui| {
                if ui
                    .add_sized([do_width, 40.0], egui::Button::new(mode.unwrap().do_label()))
                    .clicked()
                {
                    try_parse = true;
                }
                if ui
                    .add_sized([prompt_width, 40.0], egui::Button::new("Copy prompt"))
                    .clicked()
                {
                    ui.ctx().copy_text(json_parsing::prompt::prompt().to_string());
                }
            });
        } else if has_do {
            if ui
                .add_sized([total_width, 40.0], egui::Button::new(mode.unwrap().do_label()))
                .clicked()
            {
                try_parse = true;
            }
        } else if has_prompt {
            if ui
                .add_sized([total_width, 40.0], egui::Button::new("Copy prompt"))
                .clicked()
            {
                ui.ctx().copy_text(json_parsing::prompt::prompt().to_string());
            }
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_sized(
                [ui.available_width(), ui.available_height()],
                egui::TextEdit::multiline(&mut state.content).desired_width(f32::INFINITY),
            );
        });
    });

    if try_parse {
        match parse_pasted_json(&state.content) {
            Ok(payloads) => {
                state.popup_payloads = Some(payloads);
                state.popup_error = None;
            }
            Err(e) => {
                state.popup_error = Some(e.to_string());
            }
        }
    }

    if let Some(payloads) = state.popup_payloads.clone() {
        let for_confirm = payloads.clone();
        let popup_action = popup(
            ui,
            PopupParams {
                id: "import_confirm",
                title: "Confirm import",
                page_label: None,
                show_nav: false,
                copy_text: None,
                buttons: vec![
                    PopupButton { label: "Confirm".to_string(), action: PopupAction::Confirm },
                    PopupButton { label: "Cancel".to_string(), action: PopupAction::Cancel },
                ],
                body: Box::new(move |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for payload in &payloads {
                                match payload {
                                    ParsedPayload::NewTexts(texts) => {
                                        ui.heading("new_texts");
                                        for nt in texts {
                                            render_new_text(ui, nt);
                                        }
                                    }
                                    ParsedPayload::NewTemplates(templates) => {
                                        ui.heading("new_templates");
                                        for nt in templates {
                                            render_new_template(ui, nt);
                                        }
                                    }
                                    ParsedPayload::NewCategories(cats) => {
                                        ui.heading("new_categories");
                                        for c in cats {
                                            render_new_category(ui, c);
                                        }
                                    }
                                }
                                ui.separator();
                            }
                        });
                }),
            },
        );

        match popup_action {
            PopupAction::Cancel => {
                state.popup_payloads = None;
            }
            PopupAction::Confirm => {
                action = JsonAction::ImportConfirmed(for_confirm);
                state.popup_payloads = None;
            }
            _ => {}
        }
    } else if let Some(err) = state.popup_error.clone() {
        let popup_action = popup(
            ui,
            PopupParams {
                id: "import_error",
                title: "Parse error",
                page_label: None,
                show_nav: false,
                copy_text: Some(err.clone()),
                buttons: vec![PopupButton {
                    label: "Close".to_string(),
                    action: PopupAction::Close,
                }],
                body: Box::new(move |ui| {
                    ui.label(&err);
                }),
            },
        );

        if let PopupAction::Close = popup_action {
            state.popup_error = None;
        }
    }

    action
}

fn render_new_text(ui: &mut egui::Ui, nt: &NewText) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(format!("title: {}", nt.title));
        ui.label(format!("body: {}", nt.body));
        if let Some(t) = &nt.type_of_text {
            ui.label(format!("type: {}", t));
        }
        if let Some(c) = &nt.category {
            ui.label(format!("category: {}", c));
        }
        if let Some(m) = &nt.meta_category {
            ui.label(format!("meta category: {}", m));
        }
    });
}

fn render_new_category(ui: &mut egui::Ui, c: &NewCategory) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(format!("name: {}", c.name));
        ui.label(format!("type: {}", c.type_of_category));
    });
}

fn render_new_template(ui: &mut egui::Ui, nt: &NewTemplate) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(format!("title: {}", nt.title));
        ui.label(format!("content: {}", nt.content));
        if let Some(i) = &nt.instructions {
            ui.label(format!("instructions: {}", i));
        }
        if let Some(e) = &nt.example {
            ui.label(format!("example: {}", e));
        }
        if let Some(c) = &nt.category {
            ui.label(format!("category: {}", c));
        }
        if let Some(m) = &nt.meta_category {
            ui.label(format!("meta category: {}", m));
        }
    });
}
