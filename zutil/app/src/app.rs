use crate::ui_screens::{disabled, edit_text, json, projects, templates};
use db_wrapper::mascot::LiveForever;
use edit_text::create_text::ui::{CreateTextAction, CreateTextState};
use edit_text::edit_single_text::ui::{EditSingleTextAction, EditSingleTextState};
use edit_text::view_text::ui::{ViewTextAction, ViewTextState};
use edit_text::ui::{EditTextAction, EditTextState};
use templates::create_template::ui::{CreateTemplateAction, CreateTemplateState};
use templates::edit_single_category::ui::{EditSingleCategoryAction, EditSingleCategoryState};
use templates::edit_single_template::ui::{EditSingleTemplateAction, EditSingleTemplateState};
use templates::fill_template::ui::{FillTemplateAction, FillTemplateState};
use projects::ui::{ProjectsAction, ProjectsState};
use error_stuff::{is_disabled, unwrap_or_bail};
use fading_popup::ui::{FadingPopupParams, FadingPopupState, PopupScreenPosition, fading_popup};
use std::sync::mpsc::{Receiver, Sender, channel};
use json::example::ui::{JsonExampleAction, JsonExampleState, default_example_json};
use json::ui::JsonState;
use crate::components::bounce_text::ui::{BounceTextState, bounce_text_ui};
use crate::components::circle_menu::ui::CircleMenuState;
use crate::globals::{ItemKind, MenuItem};
use std::collections::{HashMap, HashSet};
use protocol::payload::DeleteRowIn;
use zutil_db::helpers::edit::{edit_text_body, edit_text_title};
use zutil_db::helpers::category::{TYPE_META, TYPE_NORMAL, category_col, category_id_opt, read_all_categories_with_ids, read_categories_with_ids_by_type, read_category_name_by_id};
use zutil_db::helpers::new_row::{new_row_template, new_row_text_with_category};
use zutil_db::helpers::project::new_row_project;
use zutil_db::helpers::edit::{edit_template_content, edit_template_example, edit_template_instructions, edit_template_title};
use zutil_db::helpers::read::{read_all_templates, read_all_texts, read_template_by_id};
use zutil_db::helpers::read::read_text_by_id;

pub fn run(db: LiveForever) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_position([0.0, 0.0])
            .with_inner_size([1920.0, 1080.0])
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "zutil",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::light());
            Ok(Box::new(App::new(db)))
        }),
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
    CircleMenu,
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
    circle_menu_state: CircleMenuState,
    bounces: Vec<BounceTextState>,
    open_template: Option<FillTemplateState>,
    editor_overlay: Option<EditorOverlay>,
}

enum EditorOverlay {
    Template(EditSingleTemplateState),
    Category(EditSingleCategoryState),
    Text(EditSingleTextState),
    ViewText(ViewTextState),
}

impl App {
    fn new(db: LiveForever) -> Self {
        crate::globals::init_menu(vec![
            MenuItem { label: "Templates".to_string(), kind: ItemKind::PushTemplatesCategories },
            MenuItem { label: "AI Prompts".to_string(), kind: ItemKind::PushPromptCategories },
            MenuItem { label: "Text".to_string(), kind: ItemKind::PushTextCategories },
        ]);
        Self {
            db,
            page: Page::CircleMenu,
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
            circle_menu_state: CircleMenuState::default(),
            bounces: Vec::new(),
            open_template: None,
            editor_overlay: None,
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

impl App {
    fn push_template_categories(&mut self) {
        let templates = match read_all_templates(&self.db) {
            Ok(t) => t,
            Err(_) => return,
        };
        let categories = match read_all_categories_with_ids(&self.db) {
            Ok(c) => c,
            Err(_) => return,
        };
        let name_by_id: HashMap<i64, String> = categories
            .into_iter()
            .map(|(id, name, _)| (id, name))
            .collect();

        let mut seen: HashSet<i64> = HashSet::new();
        let mut items: Vec<MenuItem> = Vec::new();
        for row in &templates {
            if let Some(cid) = row.cols.get(5).and_then(|c| c.as_int().ok()).copied() {
                if seen.insert(cid) {
                    if let Some(name) = name_by_id.get(&cid) {
                        items.push(MenuItem {
                            label: name.clone(),
                            kind: ItemKind::PushTemplatesInCategory(cid),
                        });
                    }
                }
            }
        }
        crate::globals::push_menu(items);
    }

    fn push_templates_in_category(&mut self, target_cid: i64) {
        let templates = match read_all_templates(&self.db) {
            Ok(t) => t,
            Err(_) => return,
        };
        let mut items: Vec<MenuItem> = Vec::new();
        for row in &templates {
            let cid = row.cols.get(5).and_then(|c| c.as_int().ok()).copied();
            if cid == Some(target_cid) {
                let id = row
                    .cols
                    .first()
                    .and_then(|c| c.as_int().ok())
                    .copied()
                    .unwrap_or(0);
                let title = row
                    .cols
                    .get(1)
                    .and_then(|c| c.as_str().ok())
                    .unwrap_or("")
                    .to_string();
                items.push(MenuItem {
                    label: title,
                    kind: ItemKind::OpenTemplate(id),
                });
            }
        }
        crate::globals::push_menu(items);
    }

    fn open_template_overlay(&mut self, tid: i64) {
        let row_opt = match read_template_by_id(&self.db, tid) {
            Ok(r) => r,
            Err(_) => return,
        };
        let row = match row_opt {
            Some(r) => r,
            None => return,
        };
        let title = row.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let content = row.cols.get(2).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let instructions = row.cols.get(3).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let example = row.cols.get(4).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();

        let mut state = FillTemplateState::default();
        state.id = tid;
        state.title = title;
        state.content = content;
        state.instructions = instructions;
        state.example = example;
        state.reset_values_for_markers();
        self.open_template = Some(state);
    }

    fn draw_template_overlay(&mut self, ui: &mut eframe::egui::Ui) {
        let mut close = false;
        if let Some(state) = self.open_template.as_mut() {
            let inner = crate::components::page_frame::ui::page_frame_opaque_ui(ui);
            ui.scope_builder(
                eframe::egui::UiBuilder::new().max_rect(inner),
                |ui| {
                    match templates::fill_template::ui::fill_template_ui(ui, state) {
                        FillTemplateAction::Back => close = true,
                        FillTemplateAction::None => {}
                    }
                },
            );
        }
        if close {
            self.open_template = None;
        }
    }
}

impl App {
    fn open_edit_template_overlay(&mut self, tid: i64) {
        let row_opt = match read_template_by_id(&self.db, tid) {
            Ok(r) => r,
            Err(_) => return,
        };
        let row = match row_opt {
            Some(r) => r,
            None => return,
        };
        let title = row.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let content = row.cols.get(2).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let instructions = row.cols.get(3).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let example = row.cols.get(4).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let category = match row.cols.get(5).and_then(|c| c.as_int().ok()).copied() {
            Some(cid) => read_category_name_by_id(&self.db, cid)
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => String::new(),
        };
        let meta_category = match row.cols.get(6).and_then(|c| c.as_int().ok()).copied() {
            Some(cid) => read_category_name_by_id(&self.db, cid)
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => String::new(),
        };
        let mut picker = crate::components::category_picker_popup::ui::CategoryPickerState::default();
        picker.current = category;
        let mut meta_picker = crate::components::category_picker_popup::ui::CategoryPickerState::default();
        meta_picker.current = meta_category;

        if let Ok((n, m)) = load_category_lists(&self.db) {
            self.available_normal_categories = n;
            self.available_meta_categories = m;
        }

        self.editor_overlay = Some(EditorOverlay::Template(EditSingleTemplateState {
            id: tid,
            title,
            content,
            instructions,
            example,
            category_picker: picker,
            meta_category_picker: meta_picker,
        }));
    }

    fn open_edit_category_overlay(&mut self, cid: i64) {
        let name = read_category_name_by_id(&self.db, cid)
            .ok()
            .flatten()
            .unwrap_or_default();
        self.editor_overlay = Some(EditorOverlay::Category(EditSingleCategoryState {
            id: cid,
            original_name: name.clone(),
            name,
        }));
    }

    fn draw_editor_overlay(&mut self, ui: &mut eframe::egui::Ui) {
        let inner = crate::components::page_frame::ui::page_frame_opaque_ui(ui);
        let mut close = false;
        let mut save: Option<EditorOverlay> = None;
        let mut text_save = false;

        let overlay = match self.editor_overlay.as_mut() {
            Some(o) => o,
            None => return,
        };

        ui.scope_builder(eframe::egui::UiBuilder::new().max_rect(inner), |ui| {
            match overlay {
                EditorOverlay::Template(state) => {
                    match templates::edit_single_template::ui::edit_single_template_ui(
                        ui,
                        state,
                        &self.available_normal_categories,
                        &self.available_meta_categories,
                    ) {
                        EditSingleTemplateAction::Back => close = true,
                        EditSingleTemplateAction::Save => {}
                        EditSingleTemplateAction::None => {}
                    }
                }
                EditorOverlay::Category(state) => {
                    match templates::edit_single_category::ui::edit_single_category_ui(ui, state) {
                        EditSingleCategoryAction::Back => close = true,
                        EditSingleCategoryAction::Save => {
                            let id = state.id;
                            let name = state.name.clone();
                            save = Some(EditorOverlay::Category(EditSingleCategoryState {
                                id,
                                original_name: name.clone(),
                                name,
                            }));
                        }
                        EditSingleCategoryAction::None => {}
                    }
                }
                EditorOverlay::Text(state) => {
                    match edit_text::edit_single_text::ui::edit_single_text_ui(
                        ui,
                        state,
                        &self.available_normal_categories,
                        &self.available_meta_categories,
                    ) {
                        EditSingleTextAction::Back => close = true,
                        EditSingleTextAction::Save => {
                            text_save = true;
                        }
                        EditSingleTextAction::None => {}
                    }
                }
                EditorOverlay::ViewText(state) => {
                    match edit_text::view_text::ui::view_text_ui(ui, state) {
                        ViewTextAction::Back => close = true,
                        ViewTextAction::None => {}
                    }
                }
            }
        });

        if text_save {
            if let Some(EditorOverlay::Text(state)) = self.editor_overlay.as_ref() {
                let id = state.id;
                let title = state.title.clone();
                let body = state.body.clone();
                let type_of_text = state.type_of_text.clone();
                let cat = state.category_picker.current.clone();
                let meta = state.meta_category_picker.current.clone();
                self.save_edit_single_text(id, title, body, type_of_text, cat, meta);
            }
        }

        if let Some(EditorOverlay::Category(state)) = &save {
            let result = self.db.edit_col_in_row(
                zutil_db::helpers::edit::edit_category_name(state.id, state.name.clone()),
            );
            unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_category", "save");
            if let Some(EditorOverlay::Category(s)) = self.editor_overlay.as_mut() {
                s.original_name = s.name.clone();
            }
        }

        if close {
            self.editor_overlay = None;
        }
    }
}

impl App {
    fn push_all_prompts(&mut self) {
        let texts = match read_all_texts(&self.db) {
            Ok(t) => t,
            Err(_) => return,
        };
        let mut items: Vec<MenuItem> = Vec::new();
        for row in &texts {
            let is_prompt = row
                .cols
                .get(5)
                .and_then(|c| c.as_str().ok())
                .map(|s| s == "prompt")
                .unwrap_or(false);
            if !is_prompt {
                continue;
            }
            let id = row.cols.first().and_then(|c| c.as_int().ok()).copied().unwrap_or(0);
            let title = row.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
            items.push(MenuItem { label: title, kind: ItemKind::CopyPrompt(id) });
        }
        crate::globals::push_menu(items);
    }

    fn push_text_categories(&mut self) {
        self.push_categories_for_kind(false);
    }

    fn push_categories_for_kind(&mut self, prompt: bool) {
        let texts = match read_all_texts(&self.db) {
            Ok(t) => t,
            Err(_) => return,
        };
        let categories = match read_all_categories_with_ids(&self.db) {
            Ok(c) => c,
            Err(_) => return,
        };
        let name_by_id: HashMap<i64, String> = categories
            .into_iter()
            .map(|(id, name, _)| (id, name))
            .collect();

        let mut seen: HashSet<i64> = HashSet::new();
        let mut items: Vec<MenuItem> = Vec::new();
        for row in &texts {
            let is_prompt = row
                .cols
                .get(5)
                .and_then(|c| c.as_str().ok())
                .map(|s| s == "prompt")
                .unwrap_or(false);
            if is_prompt != prompt {
                continue;
            }
            if let Some(cid) = row.cols.get(3).and_then(|c| c.as_int().ok()).copied() {
                if seen.insert(cid) {
                    if let Some(name) = name_by_id.get(&cid) {
                        let kind = if prompt {
                            ItemKind::PushPromptsInCategory(cid)
                        } else {
                            ItemKind::PushTextsInCategory(cid)
                        };
                        items.push(MenuItem { label: name.clone(), kind });
                    }
                }
            }
        }
        crate::globals::push_menu(items);
    }

    fn push_prompts_in_category(&mut self, target_cid: i64) {
        self.push_texts_in_category_filtered(target_cid, true);
    }

    fn push_texts_in_category(&mut self, target_cid: i64) {
        self.push_texts_in_category_filtered(target_cid, false);
    }

    fn push_texts_in_category_filtered(&mut self, target_cid: i64, prompt: bool) {
        let texts = match read_all_texts(&self.db) {
            Ok(t) => t,
            Err(_) => return,
        };
        let mut items: Vec<MenuItem> = Vec::new();
        for row in &texts {
            let is_prompt = row
                .cols
                .get(5)
                .and_then(|c| c.as_str().ok())
                .map(|s| s == "prompt")
                .unwrap_or(false);
            if is_prompt != prompt {
                continue;
            }
            let cid = row.cols.get(3).and_then(|c| c.as_int().ok()).copied();
            if cid == Some(target_cid) {
                let id = row.cols.first().and_then(|c| c.as_int().ok()).copied().unwrap_or(0);
                let title = row.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
                let kind = if prompt {
                    ItemKind::CopyPrompt(id)
                } else {
                    ItemKind::ViewText(id)
                };
                items.push(MenuItem { label: title, kind });
            }
        }
        crate::globals::push_menu(items);
    }

    fn copy_prompt(&mut self, ui: &mut eframe::egui::Ui, tid: i64) {
        let row_opt = match read_text_by_id(&self.db, tid) {
            Ok(r) => r,
            Err(_) => return,
        };
        let row = match row_opt {
            Some(r) => r,
            None => return,
        };
        let body = row.cols.get(2).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        ui.ctx().copy_text(body);

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
            id: format!("copied_{}", tid),
            title: None,
            body: "copied".to_string(),
        });
    }

    fn open_view_text_overlay(&mut self, tid: i64) {
        let row_opt = match read_text_by_id(&self.db, tid) {
            Ok(r) => r,
            Err(_) => return,
        };
        let row = match row_opt {
            Some(r) => r,
            None => return,
        };
        let title = row.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let body = row.cols.get(2).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        self.editor_overlay = Some(EditorOverlay::ViewText(ViewTextState { title, body }));
    }

    fn open_edit_text_overlay(&mut self, tid: i64) {
        let row_opt = match read_text_by_id(&self.db, tid) {
            Ok(r) => r,
            Err(_) => return,
        };
        let row = match row_opt {
            Some(r) => r,
            None => return,
        };
        let title = row.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let body = row.cols.get(2).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
        let category = match row.cols.get(3).and_then(|c| c.as_int().ok()).copied() {
            Some(cid) => read_category_name_by_id(&self.db, cid).ok().flatten().unwrap_or_default(),
            None => String::new(),
        };
        let meta_category = match row.cols.get(4).and_then(|c| c.as_int().ok()).copied() {
            Some(cid) => read_category_name_by_id(&self.db, cid).ok().flatten().unwrap_or_default(),
            None => String::new(),
        };
        let type_of_text = row.cols.get(5).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();

        let mut picker = crate::components::category_picker_popup::ui::CategoryPickerState::default();
        picker.current = category;
        let mut meta_picker = crate::components::category_picker_popup::ui::CategoryPickerState::default();
        meta_picker.current = meta_category;

        if let Ok((n, m)) = load_category_lists(&self.db) {
            self.available_normal_categories = n;
            self.available_meta_categories = m;
        }

        self.editor_overlay = Some(EditorOverlay::Text(EditSingleTextState {
            id: tid,
            title,
            body,
            type_of_text,
            category_picker: picker,
            meta_category_picker: meta_picker,
        }));
    }

    fn save_edit_single_text(
        &mut self,
        id: i64,
        title: String,
        body: String,
        type_of_text: String,
        cat: String,
        meta: String,
    ) {
        let result = self.db.edit_col_in_row(edit_text_title(id, title));
        unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_text", "edit_text_title");
        let result = self.db.edit_col_in_row(edit_text_body(id, body));
        unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_text", "edit_text_body");

        let type_value = if type_of_text.is_empty() {
            protocol::row_col::Col::Null
        } else {
            protocol::row_col::Col::Text(type_of_text)
        };
        let result = self.db.edit_col_in_row(protocol::payload::EditColInRowIn {
            table_name: "texts".to_string(),
            row_id: id.to_string(),
            column: "type_of_text".to_string(),
            new_value: type_value,
        });
        unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_text", "edit_type_of_text");

        let category_col_value = unwrap_or_bail!(
            category_col(&self.db, &cat, TYPE_NORMAL).map_err(|e| e.to_string()),
            "edit_single_text",
            "category_col"
        );
        let result = self.db.edit_col_in_row(protocol::payload::EditColInRowIn {
            table_name: "texts".to_string(),
            row_id: id.to_string(),
            column: "category_id".to_string(),
            new_value: category_col_value,
        });
        unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_text", "edit_category_id");

        let meta_col_value = unwrap_or_bail!(
            category_col(&self.db, &meta, TYPE_META).map_err(|e| e.to_string()),
            "edit_single_text",
            "category_col"
        );
        let result = self.db.edit_col_in_row(protocol::payload::EditColInRowIn {
            table_name: "texts".to_string(),
            row_id: id.to_string(),
            column: "meta_category_id".to_string(),
            new_value: meta_col_value,
        });
        unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_text", "edit_meta_category_id");
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        design::colors::BACKDROP
    }

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

        eframe::egui::CentralPanel::default()
            .frame(eframe::egui::Frame::new().fill(eframe::egui::Color32::TRANSPARENT))
            .show(ui, |ui| {
                if is_disabled() {
                    disabled::ui::disabled_ui(ui, &self.db);
                    return;
                }

                crate::globals::set_modal_open(
                    self.open_template.is_some() || self.editor_overlay.is_some(),
                );
                crate::globals::set_disable_rightclick(false);

                if ui.input(|i| i.pointer.secondary_clicked())
                    && !matches!(self.page, Page::CircleMenu)
                {
                    if self.open_template.is_some() {
                        self.open_template = None;
                    } else if self.editor_overlay.is_some() {
                        self.editor_overlay = None;
                    } else {
                        self.page = Page::CircleMenu;
                    }
                }

                let screen = ui.available_rect_before_wrap();
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    design::colors::BACKDROP_OPAQUE,
                );

                let close_center =
                    eframe::egui::pos2(
                        screen.right() - design::numbers::SCREEN_CORNER,
                        screen.top() + design::numbers::SCREEN_CORNER,
                    );
                if crate::components::close_button::ui::close_button_ui(ui, close_center) {
                    ui.ctx()
                        .send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(true));
                }

                if !crate::globals::modal_open() {
                    use crate::components::top_menu::ui::TopMenuAction;
                    match crate::components::top_menu::ui::top_menu_ui(ui) {
                        TopMenuAction::GoToViewAll => {
                            self.edit_text_state.reload(&self.db);
                            self.page = Page::EditText;
                        }
                        TopMenuAction::GoToProjects => {
                            self.projects_state.reload(&self.db);
                            self.page = Page::Projects;
                        }
                        TopMenuAction::GoToJson => {
                            self.page = Page::Json;
                        }
                        TopMenuAction::None => {}
                    }
                }

                if matches!(self.page, Page::CircleMenu) {
                    let texts = crate::globals::current_labels();
                    let response = crate::components::circle_menu::ui::circle_menu_ui(
                        ui,
                        &mut self.circle_menu_state,
                        &texts,
                    );

                    if ui.input(|i| i.pointer.secondary_clicked()) {
                        if self.open_template.is_some() {
                            self.open_template = None;
                        } else if self.editor_overlay.is_some() {
                            self.editor_overlay = None;
                        }
                    }

                    if self.open_template.is_none()
                        && self.editor_overlay.is_none()
                        && response.right_clicked.is_none()
                        && ui.input(|i| i.pointer.secondary_clicked())
                        && crate::globals::depth() > 1
                        && !crate::globals::disable_rightclick()
                    {
                        crate::globals::pop_menu();
                    }

                    if self.open_template.is_none()
                        && self.editor_overlay.is_none()
                    {
                    if let Some(i) = response.right_clicked {
                        let items = crate::globals::current_items();
                        if let Some(item) = items.get(i) {
                            match item.kind.clone() {
                                ItemKind::OpenTemplate(tid) => {
                                    self.open_edit_template_overlay(tid);
                                }
                                ItemKind::PushTemplatesInCategory(cid) => {
                                    self.open_edit_category_overlay(cid);
                                }
                                ItemKind::CopyPrompt(tid) => {
                                    self.open_edit_text_overlay(tid);
                                }
                                ItemKind::OpenTextEditor(tid) => {
                                    self.open_edit_text_overlay(tid);
                                }
                                ItemKind::ViewText(tid) => {
                                    self.open_edit_text_overlay(tid);
                                }
                                _ => {}
                            }
                        }
                    }

                    }

                    if let Some(i) = response.clicked {
                        let r = response.rects.get(i).copied().unwrap_or(eframe::egui::Rect::NOTHING);
                        if self.bounces.is_empty() && r != eframe::egui::Rect::NOTHING {
                            self.bounces.push(BounceTextState::new(
                                ui,
                                r.center(),
                                r.size(),
                                texts.get(i).cloned().unwrap_or_default(),
                            ));
                        }
                        {
                            let items = crate::globals::current_items();
                            if let Some(item) = items.get(i) {
                                match item.kind.clone() {
                                    ItemKind::PushTemplatesCategories => {
                                        self.push_template_categories();
                                    }
                                    ItemKind::PushTemplatesInCategory(cid) => {
                                        self.push_templates_in_category(cid);
                                    }
                                    ItemKind::OpenTemplate(tid) => {
                                        self.open_template_overlay(tid);
                                    }
                                    ItemKind::PushPromptCategories => {
                                        self.push_all_prompts();
                                    }
                                    ItemKind::PushPromptsInCategory(cid) => {
                                        self.push_prompts_in_category(cid);
                                    }
                                    ItemKind::CopyPrompt(tid) => {
                                        self.copy_prompt(ui, tid);
                                    }
                                    ItemKind::PushTextCategories => {
                                        self.push_text_categories();
                                    }
                                    ItemKind::PushTextsInCategory(cid) => {
                                        self.push_texts_in_category(cid);
                                    }
                                    ItemKind::OpenTextEditor(tid) => {
                                        self.open_edit_text_overlay(tid);
                                    }
                                    ItemKind::ViewText(tid) => {
                                        self.open_view_text_overlay(tid);
                                    }
                                    ItemKind::None => {}
                                }
                            }
                        }
                    }

                    if !self.bounces.is_empty() {
                        ui.ctx().request_repaint();
                        self.bounces.retain(|b| !bounce_text_ui(ui, b));
                    }

                    if self.open_template.is_none() && crate::globals::depth() > 1 {
                        let back_center = eframe::egui::pos2(
                            screen.left() + design::numbers::SCREEN_CORNER,
                            screen.top() + design::numbers::SCREEN_CORNER,
                        );
                        if crate::components::back_button::ui::back_button_ui(ui, back_center) {
                            crate::globals::pop_menu();
                        }
                    }

                    if self.open_template.is_some() {
                        self.draw_template_overlay(ui);
                    }
                    if self.editor_overlay.is_some() {
                        self.draw_editor_overlay(ui);
                    }

                    return;
                }

                let back_center =
                    eframe::egui::pos2(
                        screen.left() + design::numbers::SCREEN_CORNER,
                        screen.top() + design::numbers::SCREEN_CORNER,
                    );
                if crate::components::back_button::ui::back_button_ui(ui, back_center) {
                    self.page = Page::CircleMenu;
                }

                let inner = crate::components::page_frame::ui::page_frame_ui(ui);

                let this = &mut *self;
                ui.scope_builder(
                    eframe::egui::UiBuilder::new().max_rect(inner),
                    |ui| {
                        this.draw_current_page(ui);
                    },
                );
            });
    }
}

impl App {
    fn draw_current_page(&mut self, ui: &mut eframe::egui::Ui) {
        match self.page {
                Page::CircleMenu => {}
                Page::EditText => {
                    match edit_text::ui::edit_text_ui(ui, &self.db, &mut self.edit_text_state) {
                        EditTextAction::Back => self.page = Page::CircleMenu,
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
                        json::ui::JsonAction::Back => self.page = Page::CircleMenu,
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
                        ProjectsAction::Back => self.page = Page::CircleMenu,
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
    }
}
