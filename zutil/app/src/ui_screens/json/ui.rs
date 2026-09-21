use crate::components::text_input::ui::TextInput;
use db_wrapper::mascot::LiveForever;
use eframe::egui;
use json_parsing::new_category::NewCategory;
use json_parsing::new_template::NewTemplate;
use json_parsing::new_text::NewText;
use json_parsing::parse::{ParsedPayload, parse_pasted_json};
use popup::ui::{PopupAction, PopupButton, PopupParams, popup};
use zutil_db::helpers::import::{ImportMode, title_exists};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum JsonMode {
    Import,
    Export,
    Validate,
}

impl JsonMode {
    pub fn has_do_button(self) -> bool {
        matches!(self, JsonMode::Import | JsonMode::Export | JsonMode::Validate)
    }

    pub fn do_label(self) -> &'static str {
        match self {
            JsonMode::Import => "Import",
            JsonMode::Export => "Export",
            JsonMode::Validate => "Validate",
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
    pub popup_collisions: Vec<String>,
    pub popup_error: Option<String>,
    pub popup_info: Option<String>,
}

impl Default for JsonState {
    fn default() -> Self {
        Self {
            mode: Some(JsonMode::Import),
            content: String::new(),
            popup_payloads: None,
            popup_collisions: Vec::new(),
            popup_error: None,
            popup_info: None,
        }
    }
}

pub enum JsonAction {
    None,
    Back,
    Example,
    Do,
    ImportConfirmed(Vec<ParsedPayload>, ImportMode),
}

pub fn json_ui(ui: &mut egui::Ui, db: &LiveForever, state: &mut JsonState) -> JsonAction {
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
            TextInput::plain_multi("json_content").show(ui, &mut state.content);
        });
    });

    if try_parse {
        match state.mode {
            Some(JsonMode::Import) => {
                match parse_pasted_json(&state.content) {
                    Ok(payloads) => {
                        state.popup_collisions = collect_collisions(db, &payloads);
                        state.popup_payloads = Some(payloads);
                        state.popup_error = None;
                    }
                    Err(e) => {
                        state.popup_error = Some(e.to_string());
                    }
                }
            }
            Some(JsonMode::Export) => {
                match build_export(db) {
                    Ok(json) => {
                        state.content = json;
                        state.popup_info = Some(
                            "Exported current DB into the text field above.".to_string(),
                        );
                    }
                    Err(e) => {
                        state.popup_error = Some(e);
                    }
                }
            }
            Some(JsonMode::Validate) => {
                match parse_pasted_json(&state.content) {
                    Ok(payloads) => {
                        if payloads.is_empty() {
                            state.popup_info =
                                Some("Valid JSON, but nothing to do.".to_string());
                        } else {
                            let mut summary = String::from("Valid JSON.\n\n");
                            for p in &payloads {
                                match p {
                                    ParsedPayload::NewTexts(items) => summary.push_str(&format!(
                                        "new_texts: {}\n",
                                        items.len()
                                    )),
                                    ParsedPayload::NewTemplates(items) => summary.push_str(
                                        &format!("new_templates: {}\n", items.len()),
                                    ),
                                    ParsedPayload::NewCategories(items) => summary.push_str(
                                        &format!("new_categories: {}\n", items.len()),
                                    ),
                                }
                            }
                            state.popup_info = Some(summary);
                        }
                    }
                    Err(e) => {
                        state.popup_error = Some(e.to_string());
                    }
                }
            }
            None => {}
        }
    }

    if let Some(payloads) = state.popup_payloads.clone() {
        let for_confirm = payloads.clone();
        let collisions = state.popup_collisions.clone();
        let has_collisions = !collisions.is_empty();

        let mut buttons = Vec::new();
        if has_collisions {
            buttons.push(PopupButton {
                label: "Replace existing".to_string(),
                action: PopupAction::Replace,
            });
            buttons.push(PopupButton {
                label: "Keep both".to_string(),
                action: PopupAction::KeepBoth,
            });
        } else {
            buttons.push(PopupButton {
                label: "Confirm".to_string(),
                action: PopupAction::Confirm,
            });
        }
        buttons.push(PopupButton {
            label: "Cancel".to_string(),
            action: PopupAction::Cancel,
        });

        let body_collisions = collisions.clone();
        let popup_action = popup(
            ui,
            PopupParams {
                id: "import_confirm",
                title: "Confirm import",
                page_label: None,
                show_nav: false,
                copy_text: None,
                buttons,
                body: Box::new(move |ui| {
                    if !body_collisions.is_empty() {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} item(s) collide with existing rows by title",
                                body_collisions.len()
                            ))
                            .color(egui::Color32::from_rgb(220, 120, 120)),
                        );
                        ui.separator();
                    }
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for payload in &payloads {
                                match payload {
                                    ParsedPayload::NewTexts(texts) => {
                                        ui.heading("new_texts");
                                        for nt in texts {
                                            let collides = body_collisions
                                                .iter()
                                                .any(|k| k == &format!("text:{}", nt.title));
                                            render_new_text(ui, nt, collides);
                                        }
                                    }
                                    ParsedPayload::NewTemplates(templates) => {
                                        ui.heading("new_templates");
                                        for nt in templates {
                                            let collides = body_collisions
                                                .iter()
                                                .any(|k| k == &format!("template:{}", nt.title));
                                            render_new_template(ui, nt, collides);
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
                state.popup_collisions.clear();
            }
            PopupAction::Confirm | PopupAction::KeepBoth => {
                action = JsonAction::ImportConfirmed(for_confirm, ImportMode::KeepBoth);
                state.popup_payloads = None;
                state.popup_collisions.clear();
            }
            PopupAction::Replace => {
                action = JsonAction::ImportConfirmed(for_confirm, ImportMode::Replace);
                state.popup_payloads = None;
                state.popup_collisions.clear();
            }
            _ => {}
        }
    } else if let Some(info) = state.popup_info.clone() {
        let popup_action = popup(
            ui,
            PopupParams {
                id: "json_info",
                title: "JSON",
                page_label: None,
                show_nav: false,
                copy_text: None,
                buttons: vec![PopupButton {
                    label: "Close".to_string(),
                    action: PopupAction::Close,
                }],
                body: Box::new(move |ui| {
                    ui.label(&info);
                }),
            },
        );

        if let PopupAction::Close = popup_action {
            state.popup_info = None;
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

fn build_export(db: &LiveForever) -> Result<String, String> {
    use std::collections::HashMap;
    use zutil_db::helpers::category::read_all_categories_with_ids;
    use zutil_db::helpers::read::{read_all_templates, read_all_texts};

    let cats = read_all_categories_with_ids(db).map_err(|e| e.to_string())?;
    let name_by_id: HashMap<i64, String> = cats
        .iter()
        .map(|(id, name, _)| (*id, name.clone()))
        .collect();

    let new_cats: Vec<NewCategory> = cats
        .iter()
        .map(|(_, name, ty)| NewCategory {
            name: name.clone(),
            type_of_category: ty.clone(),
        })
        .collect();

    let text_rows = read_all_texts(db).map_err(|e| e.to_string())?;
    let new_texts: Vec<NewText> = text_rows
        .iter()
        .filter_map(|r| {
            let title = r.cols.get(1)?.as_str().ok()?.to_string();
            let body = r.cols.get(2)?.as_str().ok()?.to_string();
            let category = r
                .cols
                .get(3)
                .and_then(|c| c.as_int().ok().copied())
                .and_then(|id| name_by_id.get(&id).cloned());
            let meta_category = r
                .cols
                .get(4)
                .and_then(|c| c.as_int().ok().copied())
                .and_then(|id| name_by_id.get(&id).cloned());
            let type_of_text = r
                .cols
                .get(5)
                .and_then(|c| c.as_str().ok())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            Some(NewText {
                title,
                body,
                category,
                meta_category,
                type_of_text,
            })
        })
        .collect();

    let tpl_rows = read_all_templates(db).map_err(|e| e.to_string())?;
    let new_tpls: Vec<NewTemplate> = tpl_rows
        .iter()
        .filter_map(|r| {
            let title = r.cols.get(1)?.as_str().ok()?.to_string();
            let content = r.cols.get(2)?.as_str().ok()?.to_string();
            let instructions = r
                .cols
                .get(3)
                .and_then(|c| c.as_str().ok())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let example = r
                .cols
                .get(4)
                .and_then(|c| c.as_str().ok())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let category = r
                .cols
                .get(5)
                .and_then(|c| c.as_int().ok().copied())
                .and_then(|id| name_by_id.get(&id).cloned());
            let meta_category = r
                .cols
                .get(6)
                .and_then(|c| c.as_int().ok().copied())
                .and_then(|id| name_by_id.get(&id).cloned());
            Some(NewTemplate {
                title,
                content,
                instructions,
                example,
                category,
                meta_category,
            })
        })
        .collect();

    json_parsing::export::build_export_json(new_cats, new_texts, new_tpls)
        .map_err(|e| e.to_string())
}

fn collect_collisions(db: &LiveForever, payloads: &[ParsedPayload]) -> Vec<String> {
    let mut out = Vec::new();
    for payload in payloads {
        match payload {
            ParsedPayload::NewTexts(items) => {
                for nt in items {
                    if title_exists(db, "texts", &nt.title).unwrap_or(false) {
                        out.push(format!("text:{}", nt.title));
                    }
                }
            }
            ParsedPayload::NewTemplates(items) => {
                for nt in items {
                    if title_exists(db, "templates", &nt.title).unwrap_or(false) {
                        out.push(format!("template:{}", nt.title));
                    }
                }
            }
            ParsedPayload::NewCategories(_) => {}
        }
    }
    out
}

fn render_new_text(ui: &mut egui::Ui, nt: &NewText, collides: bool) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        if collides {
            ui.label(
                egui::RichText::new("⚠ collides with existing row")
                    .color(egui::Color32::from_rgb(220, 120, 120)),
            );
        }
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

fn render_new_template(ui: &mut egui::Ui, nt: &NewTemplate, collides: bool) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        if collides {
            ui.label(
                egui::RichText::new("⚠ collides with existing row")
                    .color(egui::Color32::from_rgb(220, 120, 120)),
            );
        }
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
