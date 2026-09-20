use crate::ui_screens::{disabled, edit_text, json, main_menu, projects, templates};
use db_wrapper::mascot::LiveForever;
use edit_text::create_text::ui::{CreateTextAction, CreateTextState};
use edit_text::edit_single_text::ui::{EditSingleTextAction, EditSingleTextState};
use edit_text::ui::{EditTextAction, EditTextState};
use templates::create_template::ui::{CreateTemplateAction, CreateTemplateState};
use templates::edit_single_template::ui::{EditSingleTemplateAction, EditSingleTemplateState};
use templates::fill_template::ui::{FillTemplateAction, FillTemplateState};
use projects::ui::{ProjectsAction, ProjectsState};
use error_stuff::{is_disabled, unwrap_or_bail};
use fading_popup::ui::{FadingPopupParams, FadingPopupState, PopupScreenPosition, fading_popup};
use std::sync::mpsc::{Receiver, Sender, channel};
use json::example::ui::{JsonExampleAction, JsonExampleState, default_example_json};
use json::ui::JsonState;
use main_menu::ui::MainMenuAction;
use protocol::payload::DeleteRowIn;
use zutil_db::helpers::edit::{edit_text_body, edit_text_title};
use zutil_db::helpers::category::{TYPE_META, TYPE_NORMAL, category_col, category_id_opt, read_categories_with_ids_by_type, read_category_name_by_id};
use zutil_db::helpers::new_row::{new_row_template, new_row_text_with_category};
use zutil_db::helpers::project::new_row_project;
use zutil_db::helpers::edit::{edit_template_content, edit_template_example, edit_template_instructions, edit_template_title};
use zutil_db::helpers::read::read_template_by_id;
use zutil_db::helpers::read::read_text_by_id;

pub fn run(db: LiveForever) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "zutil",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(db)))),
    )
}

struct FadingPopup {
    state: FadingPopupState,
    rx: Receiver<()>,
    tx: Sender<()>,
    ticker_stop: Sender<()>,
    id: String,
    title: Option<String>,
    body: String,
}

enum Page {
    MainMenu,
    EditText,
    CreateText,
    EditSingleText,
    Json,
    JsonExample,
    CreateTemplate,
    EditSingleTemplate,
    FillTemplate,
    Projects,
}

pub struct App {
    db: LiveForever,
    page: Page,
    edit_text_state: EditTextState,
    create_text_state: CreateTextState,
    edit_single_text_state: EditSingleTextState,
    json_state: JsonState,
    json_example_state: JsonExampleState,
    fading_popup: Option<FadingPopup>,
    available_normal_categories: Vec<String>,
    available_meta_categories: Vec<String>,
    create_template_state: CreateTemplateState,
    edit_single_template_state: EditSingleTemplateState,
    fill_template_state: FillTemplateState,
    projects_state: ProjectsState,
}

impl App {
    fn new(db: LiveForever) -> Self {
        Self {
            db,
            page: Page::MainMenu,
            edit_text_state: EditTextState::default(),
            create_text_state: CreateTextState::default(),
            edit_single_text_state: EditSingleTextState::default(),
            json_state: JsonState::default(),
            json_example_state: JsonExampleState::default(),
            fading_popup: None,
            available_normal_categories: Vec::new(),
            available_meta_categories: Vec::new(),
            create_template_state: CreateTemplateState::default(),
            edit_single_template_state: EditSingleTemplateState::default(),
            fill_template_state: FillTemplateState::default(),
            projects_state: ProjectsState::default(),
        }
    }

    fn open_fill_template(&mut self, id: i64) {
        let row_opt = unwrap_or_bail!(
            read_template_by_id(&self.db, id).map_err(|e| e.to_string()),
            "fill_template",
            "open_fill_template"
        );
        let row = unwrap_or_bail!(
            row_opt.ok_or("template row not found".to_string()),
            "fill_template",
            "open_fill_template"
        );
        let title = unwrap_or_bail!(
            row.cols[1].as_str().map(|s| s.to_string()),
            "fill_template",
            "open_fill_template"
        );
        let content = unwrap_or_bail!(
            row.cols[2].as_str().map(|s| s.to_string()),
            "fill_template",
            "open_fill_template"
        );
        let instructions = match row.cols.get(3).and_then(|c| c.as_str().ok()) {
            Some(s) => s.to_string(),
            None => String::new(),
        };
        let example = match row.cols.get(4).and_then(|c| c.as_str().ok()) {
            Some(s) => s.to_string(),
            None => String::new(),
        };

        let mut state = FillTemplateState::default();
        state.id = id;
        state.title = title;
        state.content = content;
        state.instructions = instructions;
        state.example = example;
        state.reset_values_for_markers();
        self.fill_template_state = state;
        self.page = Page::FillTemplate;
    }

    fn open_edit_single_template(&mut self, id: i64) {
        let row_opt = unwrap_or_bail!(
            read_template_by_id(&self.db, id).map_err(|e| e.to_string()),
            "edit_single_template",
            "open_edit_single_template"
        );
        let row = unwrap_or_bail!(
            row_opt.ok_or("template row not found".to_string()),
            "edit_single_template",
            "open_edit_single_template"
        );

        let title = unwrap_or_bail!(
            row.cols[1].as_str().map(|s| s.to_string()),
            "edit_single_template",
            "open_edit_single_template"
        );
        let content = unwrap_or_bail!(
            row.cols[2].as_str().map(|s| s.to_string()),
            "edit_single_template",
            "open_edit_single_template"
        );
        let instructions = match row.cols.get(3).and_then(|c| c.as_str().ok()) {
            Some(s) => s.to_string(),
            None => String::new(),
        };
        let example = match row.cols.get(4).and_then(|c| c.as_str().ok()) {
            Some(s) => s.to_string(),
            None => String::new(),
        };

        let category = match row.cols.get(5).and_then(|c| c.as_int().ok()).copied() {
            Some(cid) => unwrap_or_bail!(
                read_category_name_by_id(&self.db, cid).map_err(|e| e.to_string()),
                "edit_single_template",
                "open_edit_single_template"
            )
            .unwrap_or_default(),
            None => String::new(),
        };
        let meta_category = match row.cols.get(6).and_then(|c| c.as_int().ok()).copied() {
            Some(cid) => unwrap_or_bail!(
                read_category_name_by_id(&self.db, cid).map_err(|e| e.to_string()),
                "edit_single_template",
                "open_edit_single_template"
            )
            .unwrap_or_default(),
            None => String::new(),
        };

        let mut picker = crate::components::category_picker_popup::ui::CategoryPickerState::default();
        picker.current = category;
        let mut meta_picker = crate::components::category_picker_popup::ui::CategoryPickerState::default();
        meta_picker.current = meta_category;

        self.edit_single_template_state = EditSingleTemplateState {
            id,
            title,
            content,
            instructions,
            example,
            category_picker: picker,
            meta_category_picker: meta_picker,
        };
        {
        let (n, m) = unwrap_or_bail!(load_category_lists(&self.db), "edit_text", "load_category_lists");
        self.available_normal_categories = n;
        self.available_meta_categories = m;
    }
        self.page = Page::EditSingleTemplate;
    }

    fn open_edit_single_text(&mut self, id: i64) {
        let row_opt = unwrap_or_bail!(
            read_text_by_id(&self.db, id).map_err(|e| e.to_string()),
            "edit_single_text",
            "open_edit_single_text"
        );
        let row = unwrap_or_bail!(
            row_opt.ok_or("text row not found".to_string()),
            "edit_single_text",
            "open_edit_single_text"
        );

        let title = unwrap_or_bail!(
            row.cols[1].as_str().map(|s| s.to_string()),
            "edit_single_text",
            "open_edit_single_text"
        );
        let body = unwrap_or_bail!(
            row.cols[2].as_str().map(|s| s.to_string()),
            "edit_single_text",
            "open_edit_single_text"
        );

        let category = match row.cols.get(3).and_then(|c| c.as_int().ok()).copied() {
            Some(cid) => unwrap_or_bail!(
                read_category_name_by_id(&self.db, cid).map_err(|e| e.to_string()),
                "edit_single_text",
                "open_edit_single_text"
            )
            .unwrap_or_default(),
            None => String::new(),
        };
        let meta_category = match row.cols.get(4).and_then(|c| c.as_int().ok()).copied() {
            Some(cid) => unwrap_or_bail!(
                read_category_name_by_id(&self.db, cid).map_err(|e| e.to_string()),
                "edit_single_text",
                "open_edit_single_text"
            )
            .unwrap_or_default(),
            None => String::new(),
        };
        let type_of_text = match row.cols.get(5).and_then(|c| c.as_str().ok()) {
            Some(s) => s.to_string(),
            None => String::new(),
        };

        let mut picker = crate::components::category_picker_popup::ui::CategoryPickerState::default();
        picker.current = category;
        let mut meta_picker = crate::components::category_picker_popup::ui::CategoryPickerState::default();
        meta_picker.current = meta_category;

        self.edit_single_text_state = EditSingleTextState {
            id,
            title,
            body,
            type_of_text,
            category_picker: picker,
            meta_category_picker: meta_picker,
        };
        {
        let (n, m) = unwrap_or_bail!(load_category_lists(&self.db), "edit_text", "load_category_lists");
        self.available_normal_categories = n;
        self.available_meta_categories = m;
    }
        self.page = Page::EditSingleText;
    }
}


fn load_category_lists(db: &LiveForever) -> Result<(Vec<String>, Vec<String>), String> {
    let normal = read_categories_with_ids_by_type(db, TYPE_NORMAL)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|(_, n)| n)
        .collect();
    let meta = read_categories_with_ids_by_type(db, TYPE_META)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|(_, n)| n)
        .collect();
    Ok((normal, meta))
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let mut popup_finished = false;
        if let Some(p) = &mut self.fading_popup {
            if p.rx.try_recv().is_ok() {
                popup_finished = true;
            } else {
                fading_popup(
                    ui.ctx(),
                    &mut p.state,
                    FadingPopupParams {
                        id: p.id.clone(),
                        title: p.title.clone(),
                        body: p.body.clone(),
                        position: PopupScreenPosition::TopCenter { y_offset: 10.0 },
                        done_tx: p.tx.clone(),
                    },
                );
            }
        }
        if popup_finished {
            if let Some(p) = self.fading_popup.take() {
                let _ = p.ticker_stop.send(());
            }
        }

        eframe::egui::CentralPanel::default().show(ui, |ui| {
            if is_disabled() {
                disabled::ui::disabled_ui(ui, &self.db);
                return;
            }

            match self.page {
                Page::MainMenu => {
                    match main_menu::ui::main_menu_ui(ui) {
                        MainMenuAction::GoToEditText => {
                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        MainMenuAction::GoToJson => {
                            self.page = Page::Json;
                        }
                        MainMenuAction::GoToProjects => {
                            self.projects_state.reload(&self.db);
                            self.page = Page::Projects;
                        }
                        MainMenuAction::None => {}
                    }
                }
                Page::EditText => {
                    match edit_text::ui::edit_text_ui(ui, &self.db, &mut self.edit_text_state) {
                        EditTextAction::Back => self.page = Page::MainMenu,
                        EditTextAction::NewTemplate => {
                            {
        let (n, m) = unwrap_or_bail!(load_category_lists(&self.db), "edit_text", "load_category_lists");
        self.available_normal_categories = n;
        self.available_meta_categories = m;
    }
                            self.create_template_state = CreateTemplateState::default();
                            self.page = Page::CreateTemplate;
                        }
                        EditTextAction::NewText => {
                            {
        let (n, m) = unwrap_or_bail!(load_category_lists(&self.db), "edit_text", "load_category_lists");
        self.available_normal_categories = n;
        self.available_meta_categories = m;
    }
                            self.create_text_state = CreateTextState::default();
                            self.page = Page::CreateText;
                        }
                        EditTextAction::EditText(id) => self.open_edit_single_text(id),
                        EditTextAction::EditTemplate(id) => self.open_edit_single_template(id),
                        EditTextAction::FillTemplate(id) => self.open_fill_template(id),
                        EditTextAction::DeleteTemplate(id) => {
                            let result = self.db.delete_row(DeleteRowIn {
                                table_name: "templates".to_string(),
                                row_id: id.to_string(),
                            });
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_text",
                                "delete_template"
                            );
                            self.edit_text_state.reload(&self.db);
                        }
                        EditTextAction::DeleteText(id) => {
                            let result = self.db.delete_row(DeleteRowIn {
                                table_name: "texts".to_string(),
                                row_id: id.to_string(),
                            });
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_text",
                                "delete_row"
                            );
                            self.edit_text_state.reload(&self.db);
                        }
                        _ => {}
                    }
                }
                Page::CreateText => {
                    match edit_text::create_text::ui::create_text_ui(
                        ui,
                        &mut self.create_text_state,
                        &self.available_normal_categories,
                        &self.available_meta_categories,
                    ) {
                        CreateTextAction::Back => {
                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        CreateTextAction::Create => {
                            let cat = self.create_text_state.category_picker.current.clone();
                            let meta = self.create_text_state.meta_category_picker.current.clone();
                            let category_id = unwrap_or_bail!(
                                category_id_opt(&self.db, &cat, TYPE_NORMAL)
                                    .map_err(|e| e.to_string()),
                                "create_text",
                                "category_id_opt"
                            );
                            let meta_category_id = unwrap_or_bail!(
                                category_id_opt(&self.db, &meta, TYPE_META)
                                    .map_err(|e| e.to_string()),
                                "create_text",
                                "category_id_opt"
                            );
                            let type_of_text = self.create_text_state.type_of_text.clone();
                            let type_value = if type_of_text.is_empty() {
                                None
                            } else {
                                Some(type_of_text)
                            };
                            let result = self.db.insert_data(new_row_text_with_category(
                                self.create_text_state.title.clone(),
                                self.create_text_state.body.clone(),
                                category_id,
                                meta_category_id,
                                type_value,
                            ));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "create_text",
                                "insert_data"
                            );
                            self.create_text_state = CreateTextState::default();

                            let (tx, rx) = channel();
                            let (ticker_stop, ticker_rx) = channel::<()>();
                            let ctx = ui.ctx().clone();
                            std::thread::spawn(move || loop {
                                if ticker_rx.try_recv().is_ok() {
                                    break;
                                }
                                ctx.request_repaint();
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            });
                            self.fading_popup = Some(FadingPopup {
                                state: FadingPopupState::new(),
                                rx,
                                tx,
                                ticker_stop,
                                id: format!("created_{}", std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos()),
                                title: None,
                                body: "created 1 text".to_string(),
                            });
                        }
                        CreateTextAction::None => {}
                    }
                }
                Page::EditSingleText => {
                    match edit_text::edit_single_text::ui::edit_single_text_ui(
                        ui,
                        &mut self.edit_single_text_state,
                        &self.available_normal_categories,
                        &self.available_meta_categories,
                    ) {
                        EditSingleTextAction::Back => {
                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        EditSingleTextAction::Save => {
                            let id = self.edit_single_text_state.id;
                            let title = self.edit_single_text_state.title.clone();
                            let body = self.edit_single_text_state.body.clone();

                            let result = self.db.edit_col_in_row(edit_text_title(id, title));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_text",
                                "edit_text_title"
                            );

                            let result = self.db.edit_col_in_row(edit_text_body(id, body));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_text",
                                "edit_text_body"
                            );

                            let type_of_text = self.edit_single_text_state.type_of_text.clone();
                            let type_value = if type_of_text.is_empty() {
                                protocol::row_col::Col::Null
                            } else {
                                protocol::row_col::Col::Text(type_of_text)
                            };
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "texts".to_string(),
                                    row_id: id.to_string(),
                                    column: "type_of_text".to_string(),
                                    new_value: type_value,
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_text",
                                "edit_type_of_text"
                            );

                            let cat = self
                                .edit_single_text_state
                                .category_picker
                                .current
                                .clone();
                            let category_col_value = unwrap_or_bail!(
                                category_col(&self.db, &cat, TYPE_NORMAL)
                                    .map_err(|e| e.to_string()),
                                "edit_single_text",
                                "category_col"
                            );
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "texts".to_string(),
                                    row_id: id.to_string(),
                                    column: "category_id".to_string(),
                                    new_value: category_col_value,
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_text",
                                "edit_category_id"
                            );

                            let meta = self
                                .edit_single_text_state
                                .meta_category_picker
                                .current
                                .clone();
                            let meta_col_value = unwrap_or_bail!(
                                category_col(&self.db, &meta, TYPE_META)
                                    .map_err(|e| e.to_string()),
                                "edit_single_text",
                                "category_col"
                            );
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "texts".to_string(),
                                    row_id: id.to_string(),
                                    column: "meta_category_id".to_string(),
                                    new_value: meta_col_value,
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_text",
                                "edit_meta_category_id"
                            );

                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        EditSingleTextAction::None => {}
                    }
                }
                Page::Json => {
                    match json::ui::json_ui(ui, &mut self.json_state) {
                        json::ui::JsonAction::Back => self.page = Page::MainMenu,
                        json::ui::JsonAction::Do => {}
                        json::ui::JsonAction::ImportConfirmed(payloads) => {
                            let (text_count, tpl_count, _cat_count) = unwrap_or_bail!(
                                zutil_db::helpers::import::import_payloads(&self.db, payloads)
                                    .map_err(|e| e.to_string()),
                                "json",
                                "import_confirm"
                            );
                            logging::insert::log(
                                &self.db,
                                "ui",
                                "json_import",
                                &format!(
                                    "imported {} text(s), {} template(s)",
                                    text_count, tpl_count
                                ),
                            )
                            .ok();
                            self.edit_text_state.reload(&self.db);

                            let (tx, rx) = channel();
                            let (ticker_stop, ticker_rx) = channel::<()>();
                            let ctx = ui.ctx().clone();
                            std::thread::spawn(move || loop {
                                if ticker_rx.try_recv().is_ok() {
                                    break;
                                }
                                ctx.request_repaint();
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            });
                            self.fading_popup = Some(FadingPopup {
                                state: FadingPopupState::new(),
                                rx,
                                tx,
                                ticker_stop,
                                id: format!("import_{}_{}", text_count, tpl_count),
                                title: None,
                                body: format!(
                                    "created {} text(s), {} template(s)",
                                    text_count, tpl_count
                                ),
                            });
                        }
                        json::ui::JsonAction::Example => {
                            if self.json_state.mode.is_some() {
                                self.json_example_state.text = if self.json_state.content.is_empty() {
                                    default_example_json().to_string()
                                } else {
                                    self.json_state.content.clone()
                                };
                                self.json_example_state.parsed.clear();
                                self.json_example_state.error = None;
                                self.page = Page::JsonExample;
                            }
                        }
                        json::ui::JsonAction::None => {}
                    }
                }
                Page::CreateTemplate => {
                    match templates::create_template::ui::create_template_ui(
                        ui,
                        &mut self.create_template_state,
                        &self.available_normal_categories,
                        &self.available_meta_categories,
                    ) {
                        CreateTemplateAction::Back => {
                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        CreateTemplateAction::Create => {
                            let cat = self.create_template_state.category_picker.current.clone();
                            let meta = self.create_template_state.meta_category_picker.current.clone();
                            let category_id = unwrap_or_bail!(
                                category_id_opt(&self.db, &cat, TYPE_NORMAL)
                                    .map_err(|e| e.to_string()),
                                "create_template",
                                "category_id_opt"
                            );
                            let meta_category_id = unwrap_or_bail!(
                                category_id_opt(&self.db, &meta, TYPE_META)
                                    .map_err(|e| e.to_string()),
                                "create_template",
                                "category_id_opt"
                            );
                            let result = self.db.insert_data(new_row_template(
                                self.create_template_state.title.clone(),
                                self.create_template_state.content.clone(),
                                self.create_template_state.instructions.clone(),
                                self.create_template_state.example.clone(),
                                category_id,
                                meta_category_id,
                            ));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "create_template",
                                "insert_data"
                            );
                            self.create_template_state = CreateTemplateState::default();
                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        CreateTemplateAction::None => {}
                    }
                }
                Page::EditSingleTemplate => {
                    match templates::edit_single_template::ui::edit_single_template_ui(
                        ui,
                        &mut self.edit_single_template_state,
                        &self.available_normal_categories,
                        &self.available_meta_categories,
                    ) {
                        EditSingleTemplateAction::Back => {
                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        EditSingleTemplateAction::Save => {
                            let id = self.edit_single_template_state.id;
                            let title = self.edit_single_template_state.title.clone();
                            let content = self.edit_single_template_state.content.clone();

                            let result = self.db.edit_col_in_row(edit_template_title(id, title));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_template",
                                "edit_template_title"
                            );
                            let result = self.db.edit_col_in_row(edit_template_content(id, content));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_template",
                                "edit_template_content"
                            );

                            let instructions = self.edit_single_template_state.instructions.clone();
                            let result = self.db.edit_col_in_row(edit_template_instructions(id, instructions));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_template",
                                "edit_template_instructions"
                            );

                            let example = self.edit_single_template_state.example.clone();
                            let result = self.db.edit_col_in_row(edit_template_example(id, example));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_template",
                                "edit_template_example"
                            );

                            let cat = self.edit_single_template_state.category_picker.current.clone();
                            let category_col_value = unwrap_or_bail!(
                                category_col(&self.db, &cat, TYPE_NORMAL)
                                    .map_err(|e| e.to_string()),
                                "edit_single_template",
                                "category_col"
                            );
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "templates".to_string(),
                                    row_id: id.to_string(),
                                    column: "category_id".to_string(),
                                    new_value: category_col_value,
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_template",
                                "edit_category_id"
                            );

                            let meta = self.edit_single_template_state.meta_category_picker.current.clone();
                            let meta_col_value = unwrap_or_bail!(
                                category_col(&self.db, &meta, TYPE_META)
                                    .map_err(|e| e.to_string()),
                                "edit_single_template",
                                "category_col"
                            );
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "templates".to_string(),
                                    row_id: id.to_string(),
                                    column: "meta_category_id".to_string(),
                                    new_value: meta_col_value,
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "edit_single_template",
                                "edit_meta_category_id"
                            );

                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        EditSingleTemplateAction::None => {}
                    }
                }
                Page::FillTemplate => {
                    match templates::fill_template::ui::fill_template_ui(
                        ui,
                        &mut self.fill_template_state,
                    ) {
                        FillTemplateAction::Back => {
                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        FillTemplateAction::None => {}
                    }
                }
                Page::Projects => {
                    match projects::ui::projects_ui(ui, &self.db, &mut self.projects_state) {
                        ProjectsAction::Back => self.page = Page::MainMenu,
                        ProjectsAction::Create { title, path, launch_zed, launch_adstud } => {
                            let result = self.db.insert_data(new_row_project(
                                title,
                                path,
                                launch_zed,
                                launch_adstud,
                            ));
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "projects",
                                "insert_project"
                            );
                            self.projects_state.reload(&self.db);
                        }
                        ProjectsAction::Update { id, title, path, launch_zed, launch_adstud } => {
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "projects".to_string(),
                                    row_id: id.to_string(),
                                    column: "title".to_string(),
                                    new_value: protocol::row_col::Col::Text(title),
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "projects",
                                "update_title"
                            );
                            let clean_path =
                                zutil_db::helpers::project::normalize_path(&path);
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "projects".to_string(),
                                    row_id: id.to_string(),
                                    column: "path".to_string(),
                                    new_value: protocol::row_col::Col::Text(clean_path),
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "projects",
                                "update_path"
                            );
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "projects".to_string(),
                                    row_id: id.to_string(),
                                    column: "launch_zed".to_string(),
                                    new_value: protocol::row_col::Col::Integer(
                                        if launch_zed { 1 } else { 0 },
                                    ),
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "projects",
                                "update_launch_zed"
                            );
                            let result = self.db.edit_col_in_row(
                                protocol::payload::EditColInRowIn {
                                    table_name: "projects".to_string(),
                                    row_id: id.to_string(),
                                    column: "launch_adstud".to_string(),
                                    new_value: protocol::row_col::Col::Integer(
                                        if launch_adstud { 1 } else { 0 },
                                    ),
                                },
                            );
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "projects",
                                "update_launch_adstud"
                            );
                            self.projects_state.reload(&self.db);
                        }
                        ProjectsAction::Delete(id) => {
                            let result = self.db.delete_row(DeleteRowIn {
                                table_name: "projects".to_string(),
                                row_id: id.to_string(),
                            });
                            unwrap_or_bail!(
                                result.map_err(|e| e.to_string()),
                                "projects",
                                "delete_project"
                            );
                            self.projects_state.reload(&self.db);
                        }
                        ProjectsAction::OpenTerminal { path } => {
                            let expanded =
                                zutil_db::helpers::project::normalize_path(&path);
                            let spawn_result = std::process::Command::new("x-terminal-emulator")
                                .current_dir(&expanded)
                                .spawn();
                            if let Err(e) = spawn_result {
                                error_stuff::report_error(error_stuff::AppError {
                                    detail: error_stuff::ErrorDetail::Col(e.to_string()),
                                    screen: "projects".to_string(),
                                    location: "open_terminal".to_string(),
                                });
                            }
                        }
                        ProjectsAction::LaunchZed { path } => {
                            let expanded = zutil_db::helpers::project::normalize_path(&path);
                            let spawn_result = std::process::Command::new("bash")
                                .arg("-ic")
                                .arg("zed .")
                                .current_dir(&expanded)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                            if let Err(e) = spawn_result {
                                error_stuff::report_error(error_stuff::AppError {
                                    detail: error_stuff::ErrorDetail::Col(e.to_string()),
                                    screen: "projects".to_string(),
                                    location: "launch_zed".to_string(),
                                });
                            }
                        }
                        ProjectsAction::LaunchAdstud { path } => {
                            let expanded = zutil_db::helpers::project::normalize_path(&path);
                            let spawn_result = std::process::Command::new("bash")
                                .arg("-ic")
                                .arg("adstud")
                                .current_dir(&expanded)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                            if let Err(e) = spawn_result {
                                error_stuff::report_error(error_stuff::AppError {
                                    detail: error_stuff::ErrorDetail::Col(e.to_string()),
                                    screen: "projects".to_string(),
                                    location: "launch_adstud".to_string(),
                                });
                            }
                        }
                        ProjectsAction::None => {}
                    }
                }
                Page::JsonExample => {
                    match json::example::ui::json_example_ui(ui, &mut self.json_example_state) {
                        JsonExampleAction::Back => self.page = Page::Json,
                        JsonExampleAction::GoToCreateText => {
                            self.create_text_state = CreateTextState::default();
                            self.page = Page::CreateText;
                        }
                        JsonExampleAction::None => {}
                    }
                }
            }
        });
    }
}
