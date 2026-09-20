use eframe::egui;
use json_parsing::parse::{ParsedPayload, parse_pasted_json};

pub struct JsonExampleState {
    pub text: String,
    pub parsed: Vec<ParsedPayload>,
    pub error: Option<String>,
}

impl Default for JsonExampleState {
    fn default() -> Self {
        Self {
            text: String::new(),
            parsed: Vec::new(),
            error: None,
        }
    }
}

pub enum JsonExampleAction {
    None,
    Back,
    GoToCreateText,
}

pub fn default_example_json() -> &'static str {
    r#"[
  {
    "parsing_instruct": "new_texts",
    "items": [
      {
        "title": "first title",
        "body": "first body",
        "category": "examples",
        "meta_category": "docs"
      },
      {
        "title": "second title",
        "body": "second body"
      }
    ]
  }
]"#
}

pub fn json_example_ui(ui: &mut egui::Ui, state: &mut JsonExampleState) -> JsonExampleAction {
    let mut action = JsonExampleAction::None;
    let mut do_parse = false;
    let mut do_reset = false;

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            action = JsonExampleAction::Back;
        }
        ui.separator();
        ui.heading("Example");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Create manually").clicked() {
                action = JsonExampleAction::GoToCreateText;
            }
        });
    });

    ui.separator();

    if let Some(err) = state.error.clone() {
        let mut reset = false;
        ui.vertical_centered(|ui| {
            ui.heading("Parse error");
            ui.add_space(8.0);
            ui.label(&err);
            ui.add_space(12.0);
            if ui.button("Reset").clicked() {
                reset = true;
            }
        });
        if reset {
            state.error = None;
        }
    } else {
        let content_height = (ui.available_height() - 60.0).max(0.0);

        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("example_left")
                .show(&mut cols[0], |ui| {
                    ui.add_sized(
                        [ui.available_width(), content_height],
                        egui::TextEdit::multiline(&mut state.text).desired_width(f32::INFINITY),
                    );
                });
            egui::ScrollArea::vertical()
                .id_salt("example_right")
                .show(&mut cols[1], |ui| {
                    for payload in &state.parsed {
                        match payload {
                            ParsedPayload::NewTexts(texts) => {
                                ui.heading("new_texts");
                                for nt in texts {
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
                            }
                            ParsedPayload::NewCategories(cats) => {
                                ui.heading("new_categories");
                                for c in cats {
                                    egui::Frame::group(ui.style()).show(ui, |ui| {
                                        ui.label(format!("name: {}", c.name));
                                        ui.label(format!("type: {}", c.type_of_category));
                                    });
                                }
                            }
                            ParsedPayload::NewTemplates(templates) => {
                                ui.heading("new_templates");
                                for nt in templates {
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
                            }
                        }
                    }
                });
        });

        ui.add_space(8.0);
        let width = ui.available_width();
        if ui.add_sized([width, 40.0], egui::Button::new("Parse")).clicked() {
            do_parse = true;
        }
    }

    if do_parse {
        match parse_pasted_json(&state.text) {
            Ok(parsed) => {
                state.parsed = parsed;
                state.error = None;
            }
            Err(e) => {
                state.error = Some(e.to_string());
            }
        }
    }

    action
}
