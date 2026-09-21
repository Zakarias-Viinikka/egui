use eframe::egui;
use crate::ui_screens::final_nav_view_or_edit_modals;
use crate::ui_screens::{disabled, edit_text, json, projects, templates};
use db_wrapper::mascot::LiveForever;
use edit_text::create_text::ui::{CreateTextAction, CreateTextState};
use final_nav_view_or_edit_modals::edit_single_text::ui::{EditSingleTextAction, EditSingleTextState};
use final_nav_view_or_edit_modals::view_text::ui::{ViewTextAction, ViewTextState};
use edit_text::ui::{EditTextAction, EditTextState};
use templates::create_template::ui::{CreateTemplateAction, CreateTemplateState};
use final_nav_view_or_edit_modals::edit_single_category::ui::{EditSingleCategoryAction, EditSingleCategoryState};
use final_nav_view_or_edit_modals::edit_single_template::ui::{EditSingleTemplateAction, EditSingleTemplateState};
use final_nav_view_or_edit_modals::fill_template::ui::{FillTemplateAction, FillTemplateState};
use projects::ui::{ProjectsAction, ProjectsState};
use error_stuff::{is_disabled, unwrap_or_bail};
use fading_popup::ui::{FadingPopupParams, FadingPopupState, PopupScreenPosition, fading_popup};
use std::sync::mpsc::{Receiver, Sender, channel};
use json::example::ui::{JsonExampleAction, JsonExampleState, default_example_json};
use json::ui::JsonState;
use crate::components::bounce_text::ui::{BounceTextState, bounce_text_ui};
use crate::components::circle_menu::ui::CircleMenuState;
use crate::components::shortcut_picker::ui::{ShortcutPickerAction, ShortcutPickerState, shortcut_picker_ui};
use crate::globals::{ItemKind, MenuItem};
use std::collections::{HashMap, HashSet};
use protocol::payload::DeleteRowIn;
use zutil_db::helpers::edit::{edit_text_body, edit_text_title};
use zutil_db::helpers::category::{TYPE_META, TYPE_NORMAL, category_col, category_id_opt, read_all_categories_with_ids, read_categories_with_ids_by_type, read_category_name_by_id};
use zutil_db::helpers::new_row::{new_row_template, new_row_text_with_category};
use zutil_db::helpers::project::new_row_project;
use zutil_db::helpers::edit::{edit_template_content, edit_template_example, edit_template_instructions, edit_template_title};
use zutil_db::helpers::popularity;
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
    shortcut_picker: Option<ShortcutPickerState>,
    last_logged_error: Option<String>,
}

enum EditorOverlay {
    Template(EditSingleTemplateState),
    Category(EditSingleCategoryState),
    Text(EditSingleTextState),
    ViewText(ViewTextState),
    Projects,
    NewPlus,
}

impl App {
    fn new(db: LiveForever) -> Self {
        let mut app = Self {
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
            shortcut_picker: None,
            last_logged_error: None,
        };
        app.init_root_menu();
        app
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
    fn read_shortcut_for(&self, kind: &'static str, id: &str) -> Option<String> {
        zutil_db::helpers::shortcuts::read_shortcut(&self.db, kind, id)
            .ok()
            .flatten()
    }

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
                        let key = cid.to_string();
                        items.push(MenuItem {
                            label: name.clone(),
                            kind: ItemKind::PushTemplatesInCategory(cid),
                            counter: popularity::read_counter(&self.db, "categories", cid)
                                .unwrap_or(0),
                            shortcut: self.read_shortcut_for(
                                zutil_db::helpers::shortcuts::OWNER_CATEGORY,
                                &key,
                            ),
                            owner: (zutil_db::helpers::shortcuts::OWNER_CATEGORY, key),
                        });
                    }
                }
            }
        }
        crate::globals::push_menu(crate::globals::RebuildKind::TemplatesCategories, items);
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
                let key = id.to_string();
                items.push(MenuItem {
                    label: title,
                    kind: ItemKind::OpenTemplate(id),
                    counter: popularity::read_counter(&self.db, "templates", id).unwrap_or(0),
                    shortcut: self.read_shortcut_for(
                        zutil_db::helpers::shortcuts::OWNER_TEMPLATE,
                        &key,
                    ),
                    owner: (zutil_db::helpers::shortcuts::OWNER_TEMPLATE, key),
                });
            }
        }
        crate::globals::push_menu(
            crate::globals::RebuildKind::TemplatesInCategory(target_cid),
            items,
        );
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
                    match final_nav_view_or_edit_modals::fill_template::ui::fill_template_ui(ui, state) {
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
        let mut template_save = false;
        let mut view_text_copy = false;
        let mut new_plus: Option<u8> = None;

        let overlay = match self.editor_overlay.as_mut() {
            Some(o) => o,
            None => return,
        };

        ui.scope_builder(eframe::egui::UiBuilder::new().max_rect(inner), |ui| {
            eframe::egui::ScrollArea::vertical().show(ui, |ui| {
            match overlay {
                EditorOverlay::Template(state) => {
                    match final_nav_view_or_edit_modals::edit_single_template::ui::edit_single_template_ui(
                        ui,
                        state,
                        &self.available_normal_categories,
                        &self.available_meta_categories,
                    ) {
                        EditSingleTemplateAction::Back => close = true,
                        EditSingleTemplateAction::Save => {
                            template_save = true;
                        }
                        EditSingleTemplateAction::None => {}
                    }
                }
                EditorOverlay::Category(state) => {
                    match final_nav_view_or_edit_modals::edit_single_category::ui::edit_single_category_ui(ui, state) {
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
                    match final_nav_view_or_edit_modals::edit_single_text::ui::edit_single_text_ui(
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
                    match final_nav_view_or_edit_modals::view_text::ui::view_text_ui(ui, state) {
                        ViewTextAction::Back => close = true,
                        ViewTextAction::Copy => {
                            ui.ctx().copy_text(state.body.clone());
                            view_text_copy = true;
                        }
                        ViewTextAction::None => {}
                    }
                }
                EditorOverlay::NewPlus => {
                    let rect = ui.available_rect_before_wrap();
                    let btn_w = 260.0_f32;
                    let btn_h = 50.0_f32;
                    let gap = 14.0_f32;
                    let total_h = btn_h * 3.0 + gap * 2.0;
                    let start_x = rect.center().x - btn_w / 2.0;
                    let mut y = rect.center().y - total_h / 2.0;
                    for (idx, label) in ["New Text", "New Template", "New Project"].iter().enumerate() {
                        let r = egui::Rect::from_min_size(
                            egui::pos2(start_x, y),
                            egui::vec2(btn_w, btn_h),
                        );
                        if ui.put(r, egui::Button::new(*label)).clicked() {
                            new_plus = Some(idx as u8);
                        }
                        y += btn_h + gap;
                    }
                }
                EditorOverlay::Projects => {
                    match projects::ui::projects_ui(ui, &self.db, &mut self.projects_state) {
                        ProjectsAction::Back => close = true,
                        ProjectsAction::Create { title, path, launch_zed, launch_adstud } => {
                            let result = self.db.insert_data(new_row_project(
                                title, path, launch_zed, launch_adstud,
                            ));
                            unwrap_or_bail!(result.map_err(|e| e.to_string()), "projects", "insert_project");
                            self.projects_state.reload(&self.db);
                        }
                        ProjectsAction::Update { id, title, path, launch_zed, launch_adstud } => {
                            let result = self.db.edit_col_in_row(protocol::payload::EditColInRowIn {
                                table_name: "projects".to_string(),
                                row_id: id.to_string(),
                                column: "title".to_string(),
                                new_value: protocol::row_col::Col::Text(title),
                            });
                            unwrap_or_bail!(result.map_err(|e| e.to_string()), "projects", "update_title");
                            let clean_path = zutil_db::helpers::project::normalize_path(&path);
                            let result = self.db.edit_col_in_row(protocol::payload::EditColInRowIn {
                                table_name: "projects".to_string(),
                                row_id: id.to_string(),
                                column: "path".to_string(),
                                new_value: protocol::row_col::Col::Text(clean_path),
                            });
                            unwrap_or_bail!(result.map_err(|e| e.to_string()), "projects", "update_path");
                            for (col, val) in [
                                ("launch_zed", launch_zed),
                                ("launch_adstud", launch_adstud),
                            ] {
                                let result = self.db.edit_col_in_row(protocol::payload::EditColInRowIn {
                                    table_name: "projects".to_string(),
                                    row_id: id.to_string(),
                                    column: col.to_string(),
                                    new_value: protocol::row_col::Col::Integer(if val { 1 } else { 0 }),
                                });
                                unwrap_or_bail!(result.map_err(|e| e.to_string()), "projects", "update_flag");
                            }
                            self.projects_state.reload(&self.db);
                        }
                        ProjectsAction::Delete(id) => {
                            let result = self.db.delete_row(DeleteRowIn {
                                table_name: "projects".to_string(),
                                row_id: id.to_string(),
                            });
                            unwrap_or_bail!(result.map_err(|e| e.to_string()), "projects", "delete_project");
                            self.projects_state.reload(&self.db);
                        }
                        ProjectsAction::OpenTerminal { path } => {
                            let expanded = zutil_db::helpers::project::normalize_path(&path);
                            let _ = std::process::Command::new("x-terminal-emulator").current_dir(&expanded).spawn();
                        }
                        ProjectsAction::LaunchZed { path } => {
                            let expanded = zutil_db::helpers::project::normalize_path(&path);
                            let _ = std::process::Command::new("bash")
                                .arg("-ic").arg("zed .")
                                .current_dir(&expanded)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                        }
                        ProjectsAction::LaunchAdstud { path } => {
                            let expanded = zutil_db::helpers::project::normalize_path(&path);
                            let _ = std::process::Command::new("bash")
                                .arg("-ic").arg("adstud")
                                .current_dir(&expanded)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                        }
                        ProjectsAction::None => {}
                    }
                }
            }
            });
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
            close = true;
            self.rebuild_menu_stack();
        }

        if template_save {
            if let Some(EditorOverlay::Template(state)) = self.editor_overlay.as_ref() {
                let id = state.id;
                let title = state.title.clone();
                let content = state.content.clone();
                let instructions = state.instructions.clone();
                let example = state.example.clone();
                let cat = state.category_picker.current.clone();
                let meta = state.meta_category_picker.current.clone();

                let result = self.db.edit_col_in_row(edit_template_title(id, title));
                unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_template", "edit_template_title");
                let result = self.db.edit_col_in_row(edit_template_content(id, content));
                unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_template", "edit_template_content");
                let result = self.db.edit_col_in_row(edit_template_instructions(id, instructions));
                unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_template", "edit_template_instructions");
                let result = self.db.edit_col_in_row(edit_template_example(id, example));
                unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_template", "edit_template_example");

                let category_col_value = unwrap_or_bail!(
                    category_col(&self.db, &cat, TYPE_NORMAL).map_err(|e| e.to_string()),
                    "edit_single_template",
                    "category_col"
                );
                let result = self.db.edit_col_in_row(protocol::payload::EditColInRowIn {
                    table_name: "templates".to_string(),
                    row_id: id.to_string(),
                    column: "category_id".to_string(),
                    new_value: category_col_value,
                });
                unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_template", "edit_category_id");

                let meta_col_value = unwrap_or_bail!(
                    category_col(&self.db, &meta, TYPE_META).map_err(|e| e.to_string()),
                    "edit_single_template",
                    "category_col"
                );
                let result = self.db.edit_col_in_row(protocol::payload::EditColInRowIn {
                    table_name: "templates".to_string(),
                    row_id: id.to_string(),
                    column: "meta_category_id".to_string(),
                    new_value: meta_col_value,
                });
                unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_single_template", "edit_meta_category_id");
            }
            close = true;
            self.rebuild_menu_stack();
        }

        if let Some(EditorOverlay::Category(state)) = &save {
            let result = self.db.edit_col_in_row(
                zutil_db::helpers::edit::edit_category_name(state.id, state.name.clone()),
            );
            unwrap_or_bail!(result.map_err(|e| e.to_string()), "edit_category", "save");
            close = true;
            self.rebuild_menu_stack();
        }

        if view_text_copy {
            self.show_copied_popup(ui, "view_text");
        }

        if let Some(idx) = new_plus {
            self.editor_overlay = None;
            match idx {
                0 => self.page = Page::CreateText,
                1 => self.page = Page::CreateTemplate,
                2 => {
                    self.projects_state.reload(&self.db);
                    self.projects_state.editing_id = None;
                    self.projects_state.form_title.clear();
                    self.projects_state.form_path.clear();
                    self.projects_state.form_launch_zed = false;
                    self.projects_state.form_launch_adstud = false;
                    self.projects_state.show_modal = true;
                    self.editor_overlay = Some(EditorOverlay::Projects);
                }
                _ => {}
            }
        }

        if close {
            self.editor_overlay = None;
        }
    }

    fn show_copied_popup(&mut self, ui: &mut eframe::egui::Ui, tag: &str) {
        self.show_popup(ui, &format!("copied_{}", tag), "copied");
    }

    fn show_popup(&mut self, ui: &mut eframe::egui::Ui, id: &str, body: &str) {
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
            id: id.to_string(),
            title: None,
            body: body.to_string(),
        });
    }

    fn log_error_once(&mut self, _ui: &mut eframe::egui::Ui) {
        if let Some(err) = error_stuff::current_error() {
            let detail = match &err.detail {
                error_stuff::ErrorDetail::Db(e) => format!("db: {}", e),
                error_stuff::ErrorDetail::Col(s) => s.clone(),
            };
            let key = format!("{}::{}::{}", err.screen, err.location, detail);
            if self.last_logged_error.as_deref() == Some(key.as_str()) {
                return;
            }
            let _ = zutil_db::helpers::error_log::insert_error(
                &self.db,
                &err.screen,
                &err.location,
                &detail,
            );
            self.last_logged_error = Some(key);
        }
    }

    fn notify_error(&mut self, ui: &mut eframe::egui::Ui, msg: String) {
        // clipboard (may or may not stick depending on WM)
        ui.ctx().copy_text(msg.clone());

        // also write to a file — this always works
        let path = "/tmp/zutil_last_error.txt";
        let _ = std::fs::write(path, &msg);

        let tag = format!("err_{}", msg.len());
        self.show_popup(ui, &tag, &msg);
    }
}

impl App {
    fn rebuild_menu_stack(&mut self) {
        use crate::globals::RebuildKind;
        let kinds = crate::globals::stack_kinds();
        if kinds.is_empty() {
            return;
        }
        for kind in kinds {
            match kind {
                RebuildKind::Root => self.init_root_menu(),
                RebuildKind::TemplatesCategories => self.push_template_categories(),
                RebuildKind::TemplatesInCategory(cid) => self.push_templates_in_category(cid),
                RebuildKind::Prompts => self.push_all_prompts(),
                RebuildKind::TextCategories => self.push_text_categories(),
                RebuildKind::TextsInCategory(cid) => self.push_texts_in_category(cid),
                RebuildKind::Projects => self.push_projects(),
                RebuildKind::TerminalCategories => self.push_terminal_categories(),
                RebuildKind::TerminalsInCategory(cid) => self.push_terminals_in_category(cid),
            }
        }
    }

    // NOTE: If you add, rename, or remove a main-nav item here, you MUST also
    // update MAIN_NAV_NAMES in db/src/helpers/popularity.rs. init_db uses that
    // list to pre-create the main_nav_clicks rows, and every counter lookup is
    // keyed by the label you pass to mk_root below.
    fn init_root_menu(&mut self) {
        let mk_root = |label: &str, kind: ItemKind| MenuItem {
            label: label.to_string(),
            kind,
            counter: popularity::read_main_nav_counter(&self.db, label).unwrap_or(0) as u32,
            shortcut: zutil_db::helpers::shortcuts::read_shortcut(
                &self.db,
                zutil_db::helpers::shortcuts::OWNER_MAIN_NAV,
                label,
            )
            .ok()
            .flatten(),
            owner: (zutil_db::helpers::shortcuts::OWNER_MAIN_NAV, label.to_string()),
        };
        crate::globals::init_menu(vec![
            mk_root("Templates", ItemKind::PushTemplatesCategories),
            mk_root("AI Prompts", ItemKind::PushPromptCategories),
            mk_root("Text", ItemKind::PushTextCategories),
            mk_root("Terminal Commands", ItemKind::PushTerminalCategories),
            mk_root("Projects", ItemKind::PushProjects),
        ]);
    }

    fn push_projects(&mut self) {
        self.projects_state.reload(&self.db);
        let items: Vec<MenuItem> = self
            .projects_state
            .rows
            .iter()
            .map(|r| {
                let key = r.id.to_string();
                MenuItem {
                    label: r.title.clone(),
                    kind: ItemKind::OpenProjectTerminal(r.id),
                    counter: popularity::read_counter(&self.db, "projects", r.id).unwrap_or(0),
                    shortcut: self.read_shortcut_for(
                        zutil_db::helpers::shortcuts::OWNER_PROJECT,
                        &key,
                    ),
                    owner: (zutil_db::helpers::shortcuts::OWNER_PROJECT, key),
                }
            })
            .collect();
        crate::globals::push_menu(crate::globals::RebuildKind::Projects, items);
    }

    fn open_project_terminal(&mut self, id: i64) {
        let row = self.projects_state.rows.iter().find(|r| r.id == id);
        if let Some(r) = row {
            let expanded = zutil_db::helpers::project::normalize_path(&r.path);
            let _ = std::process::Command::new("x-terminal-emulator")
                .current_dir(&expanded)
                .spawn();
        }
    }

    fn edit_project_overlay(&mut self, id: i64) {
        self.projects_state.reload(&self.db);
        if let Some(row) = self.projects_state.rows.iter().find(|r| r.id == id) {
            self.projects_state.editing_id = Some(row.id);
            self.projects_state.form_title = row.title.clone();
            self.projects_state.form_path = row.path.clone();
            self.projects_state.form_launch_zed = row.launch_zed;
            self.projects_state.form_launch_adstud = row.launch_adstud;
            self.projects_state.show_modal = true;
        }
        self.editor_overlay = Some(EditorOverlay::Projects);
    }

    fn open_projects_overlay(&mut self) {
        self.projects_state.reload(&self.db);
        self.editor_overlay = Some(EditorOverlay::Projects);
    }

    fn dispatch_by_kind(&mut self, ui: &mut eframe::egui::Ui, item: &MenuItem) {
        match item.kind.clone() {
            ItemKind::PushTemplatesCategories => self.push_template_categories(),
            ItemKind::PushTemplatesInCategory(cid) => self.push_templates_in_category(cid),
            ItemKind::OpenTemplate(tid) => self.open_template_overlay(tid),
            ItemKind::PushPromptCategories => self.push_all_prompts(),
            ItemKind::PushPromptsInCategory(cid) => self.push_prompts_in_category(cid),
            ItemKind::CopyPrompt(tid) => self.copy_prompt(ui, tid),
            ItemKind::PushTextCategories => self.push_text_categories(),
            ItemKind::PushTextsInCategory(cid) => self.push_texts_in_category(cid),
            ItemKind::OpenTextEditor(tid) => self.open_edit_text_overlay(tid),
            ItemKind::ViewText(tid) => self.open_view_text_overlay(tid),
            ItemKind::PushProjects => self.push_projects(),
            ItemKind::OpenProjectTerminal(id) => self.open_project_terminal(id),
            ItemKind::EditProject(id) => self.edit_project_overlay(id),
            ItemKind::NewPlus => self.editor_overlay = Some(EditorOverlay::NewPlus),
            ItemKind::PushTerminalCategories => self.push_terminal_categories(),
            ItemKind::PushTerminalsInCategory(cid) => self.push_terminals_in_category(cid),
            ItemKind::None => {}
        }
    }

    fn bump_counter_for(&mut self, item: &MenuItem) {
        let r = match &item.kind {
            ItemKind::PushTemplatesCategories
            | ItemKind::PushPromptCategories
            | ItemKind::PushTextCategories
            | ItemKind::PushTerminalCategories
            | ItemKind::PushProjects => {
                popularity::increment_main_nav(&self.db, &item.label)
            }
            ItemKind::PushTemplatesInCategory(cid)
            | ItemKind::PushPromptsInCategory(cid)
            | ItemKind::PushTextsInCategory(cid)
            | ItemKind::PushTerminalsInCategory(cid) => {
                popularity::increment_category(&self.db, *cid)
            }
            ItemKind::OpenTemplate(id) => popularity::increment_template(&self.db, *id),
            ItemKind::CopyPrompt(id)
            | ItemKind::ViewText(id)
            | ItemKind::OpenTextEditor(id) => popularity::increment_text(&self.db, *id),
            ItemKind::OpenProjectTerminal(id) | ItemKind::EditProject(id) => {
                popularity::increment_project(&self.db, *id)
            }
            _ => return,
        };
        if r.is_ok() {
            self.rebuild_menu_stack();
        }
    }

    fn draw_shortcut_picker(&mut self, ui: &mut eframe::egui::Ui) {
        let mut done: Option<ShortcutPickerAction> = None;
        if let Some(state) = self.shortcut_picker.as_mut() {
            let action = shortcut_picker_ui(ui, state);
            if !matches!(action, ShortcutPickerAction::None) {
                done = Some(action);
            }
        }
        match done {
            Some(ShortcutPickerAction::Cancel) => {
                self.shortcut_picker = None;
            }
            Some(ShortcutPickerAction::Save { owner_kind, owner_id, combo }) => {
                // reject if another item in the *current* circle already uses it
                let items = crate::globals::current_items();
                let mut taken = false;
                for item in &items {
                    if item.owner.0 == owner_kind && item.owner.1 == owner_id {
                        continue;
                    }
                    if item.shortcut.as_deref() == Some(combo.as_str()) {
                        taken = true;
                        break;
                    }
                }
                if taken {
                    let msg = "already used in this menu".to_string();
                    self.notify_error(ui, msg.clone());
                    if let Some(state) = self.shortcut_picker.as_mut() {
                        state.error = Some(msg);
                    }
                } else {
                    let res = zutil_db::helpers::shortcuts::set_shortcut(
                        &self.db,
                        owner_kind,
                        &owner_id,
                        &combo,
                    );
                    match res {
                        Ok(()) => {
                            self.shortcut_picker = None;
                            self.rebuild_menu_stack();
                        }
                        Err(e) => {
                            let msg = format!("save failed: {}", e);
                            self.notify_error(ui, msg.clone());
                            if let Some(state) = self.shortcut_picker.as_mut() {
                                state.error = Some(msg);
                            }
                        }
                    }
                }
            }
            None => {}
            Some(ShortcutPickerAction::None) => {}
        }
    }

    fn try_close_innermost_overlay(&mut self) -> bool {
        // projects editor has an inner modal — close that first
        if matches!(self.editor_overlay, Some(EditorOverlay::Projects))
            && self.projects_state.show_modal
        {
            self.projects_state.show_modal = false;
            return true;
        }
        if self.open_template.is_some() {
            self.open_template = None;
            return true;
        }
        if self.editor_overlay.is_some() {
            self.editor_overlay = None;
            return true;
        }
        false
    }

    fn push_terminal_categories(&mut self) {
        self.push_categories_for_type("terminal command", ItemKind::PushTerminalsInCategory);
    }

    fn push_terminals_in_category(&mut self, cid: i64) {
        self.push_texts_in_category_by_type(cid, "terminal command", ItemKind::CopyPrompt);
    }

    fn push_categories_for_type(
        &mut self,
        type_of_text: &str,
        mk_kind: fn(i64) -> ItemKind,
    ) {
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
            let t = row.cols.get(5).and_then(|c| c.as_str().ok()).unwrap_or("");
            if t != type_of_text {
                continue;
            }
            if let Some(cid) = row.cols.get(3).and_then(|c| c.as_int().ok()).copied() {
                if seen.insert(cid) {
                    if let Some(name) = name_by_id.get(&cid) {
                        let key = cid.to_string();
                        items.push(MenuItem {
                            label: name.clone(),
                            kind: mk_kind(cid),
                            counter: popularity::read_counter(&self.db, "categories", cid)
                                .unwrap_or(0),
                            shortcut: self.read_shortcut_for(
                                zutil_db::helpers::shortcuts::OWNER_CATEGORY,
                                &key,
                            ),
                            owner: (zutil_db::helpers::shortcuts::OWNER_CATEGORY, key),
                        });
                    }
                }
            }
        }
        let kind = match type_of_text {
            "prompt" => crate::globals::RebuildKind::Prompts,
            "terminal command" => crate::globals::RebuildKind::TerminalCategories,
            _ => crate::globals::RebuildKind::TextCategories,
        };
        crate::globals::push_menu(kind, items);
    }

    fn push_texts_in_category_by_type(
        &mut self,
        target_cid: i64,
        type_of_text: &str,
        mk_kind: fn(i64) -> ItemKind,
    ) {
        let texts = match read_all_texts(&self.db) {
            Ok(t) => t,
            Err(_) => return,
        };
        let mut items: Vec<MenuItem> = Vec::new();
        for row in &texts {
            let t = row.cols.get(5).and_then(|c| c.as_str().ok()).unwrap_or("");
            if t != type_of_text {
                continue;
            }
            let cid = row.cols.get(3).and_then(|c| c.as_int().ok()).copied();
            if cid == Some(target_cid) {
                let id = row.cols.first().and_then(|c| c.as_int().ok()).copied().unwrap_or(0);
                let title = row.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
                let key = id.to_string();
                items.push(MenuItem {
                    label: title,
                    kind: mk_kind(id),
                    counter: popularity::read_counter(&self.db, "texts", id).unwrap_or(0),
                    shortcut: self.read_shortcut_for(
                        zutil_db::helpers::shortcuts::OWNER_TEXT,
                        &key,
                    ),
                    owner: (zutil_db::helpers::shortcuts::OWNER_TEXT, key),
                });
            }
        }
        let kind = match type_of_text {
            "prompt" => crate::globals::RebuildKind::Prompts,
            "terminal command" => crate::globals::RebuildKind::TerminalsInCategory(target_cid),
            _ => crate::globals::RebuildKind::TextsInCategory(target_cid),
        };
        crate::globals::push_menu(kind, items);
    }

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
            let key = id.to_string();
            items.push(MenuItem {
                label: title,
                kind: ItemKind::CopyPrompt(id),
                counter: popularity::read_counter(&self.db, "texts", id).unwrap_or(0),
                shortcut: self.read_shortcut_for(
                    zutil_db::helpers::shortcuts::OWNER_TEXT,
                    &key,
                ),
                owner: (zutil_db::helpers::shortcuts::OWNER_TEXT, key),
            });
        }
        crate::globals::push_menu(crate::globals::RebuildKind::Prompts, items);
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
                        let key = cid.to_string();
                        items.push(MenuItem {
                            label: name.clone(),
                            kind,
                            counter: popularity::read_counter(&self.db, "categories", cid)
                                .unwrap_or(0),
                            shortcut: self.read_shortcut_for(
                                zutil_db::helpers::shortcuts::OWNER_CATEGORY,
                                &key,
                            ),
                            owner: (zutil_db::helpers::shortcuts::OWNER_CATEGORY, key),
                        });
                    }
                }
            }
        }
        let kind = if prompt {
            crate::globals::RebuildKind::Prompts
        } else {
            crate::globals::RebuildKind::TextCategories
        };
        crate::globals::push_menu(kind, items);
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
                let key = id.to_string();
                items.push(MenuItem {
                    label: title,
                    kind,
                    counter: popularity::read_counter(&self.db, "texts", id).unwrap_or(0),
                    shortcut: self.read_shortcut_for(
                        zutil_db::helpers::shortcuts::OWNER_TEXT,
                        &key,
                    ),
                    owner: (zutil_db::helpers::shortcuts::OWNER_TEXT, key),
                });
            }
        }
        let kind = if prompt {
            crate::globals::RebuildKind::Prompts
        } else {
            crate::globals::RebuildKind::TextsInCategory(target_cid)
        };
        crate::globals::push_menu(kind, items);
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
        self.show_copied_popup(ui, &format!("prompt_{}", tid));
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
                    self.log_error_once(ui);
                    disabled::ui::disabled_ui(ui, &self.db);
                    return;
                }

                crate::globals::set_modal_open(
                    self.open_template.is_some()
                        || self.editor_overlay.is_some()
                        || self.shortcut_picker.is_some(),
                );

                crate::globals::set_disable_rightclick(false);

                if ui.input(|i| i.pointer.secondary_clicked())
                    && !matches!(self.page, Page::CircleMenu)
                    && !self.try_close_innermost_overlay()
                {
                    self.page = Page::CircleMenu;
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
                        TopMenuAction::NewPlus => {
                            self.editor_overlay = Some(EditorOverlay::NewPlus);
                        }
                        TopMenuAction::HalveCounters => {
                            let _ = popularity::halve_all(&self.db);
                            self.rebuild_menu_stack();
                        }
                        TopMenuAction::None => {}
                    }
                }

                if matches!(self.page, Page::CircleMenu) {
                    if !crate::globals::modal_open()
                        && self.shortcut_picker.is_none()
                        && self.open_template.is_none()
                        && self.editor_overlay.is_none()
                    {
                        let mut fired: Option<usize> = None;
                        let current = crate::globals::current_items();
                        ui.input(|i| {
                            for ev in &i.events {
                                if let eframe::egui::Event::Key {
                                    key,
                                    pressed: true,
                                    modifiers,
                                    ..
                                } = ev
                                {
                                    if crate::components::shortcut_picker::ui::is_modifier(*key) {
                                        continue;
                                    }
                                    let combo =
                                        crate::components::shortcut_picker::ui::build_combo(
                                            *key, modifiers,
                                        );
                                    for (idx, it) in current.iter().enumerate() {
                                        if it.shortcut.as_deref() == Some(combo.as_str()) {
                                            fired = Some(idx);
                                            break;
                                        }
                                    }
                                    if fired.is_some() {
                                        break;
                                    }
                                }
                            }
                        });
                        if let Some(i) = fired {
                            // reuse the same code path as a click
                            let item = current[i].clone();
                            self.bump_counter_for(&item);
                            self.dispatch_by_kind(ui, &item);
                            return;
                        }
                    }

                    let items = crate::globals::current_items();
                    let response = crate::components::circle_menu::ui::circle_menu_ui(
                        ui,
                        &mut self.circle_menu_state,
                        &items,
                    );

                    let secondary = ui.input(|i| i.pointer.secondary_clicked());
                    if secondary && self.try_close_innermost_overlay() {
                        // consumed
                    } else if secondary
                        && response.right_clicked.is_none()
                        && crate::globals::depth() > 1
                        && !crate::globals::disable_rightclick()
                    {
                        crate::globals::pop_menu();
                    }

                    if self.open_template.is_none()
                        && self.editor_overlay.is_none()
                    {
                    let shortcut_open = response
                        .clicked_shortcut
                        .or(response.right_clicked_shortcut);
                    if let Some(i) = shortcut_open {
                        if let Some(item) = items.get(i) {
                            let existing = item.shortcut.clone();
                            self.shortcut_picker = Some(ShortcutPickerState {
                                owner_kind: item.owner.0,
                                owner_id: item.owner.1.clone(),
                                owner_label: item.label.clone(),
                                combo: existing,
                                error: None,
                            });
                        }
                    }

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
                                ItemKind::OpenProjectTerminal(id) => {
                                    self.edit_project_overlay(id);
                                }
                                _ => {}
                            }
                        }
                    }

                    }

                    if let Some(i) = response.clicked {
                        let r = response.rects.get(i).copied().unwrap_or(eframe::egui::Rect::NOTHING);
                        let clicked_label = items.get(i).map(|it| it.label.clone()).unwrap_or_default();
                        if self.bounces.is_empty() && r != eframe::egui::Rect::NOTHING {
                            self.bounces.push(BounceTextState::new(
                                ui,
                                r.center(),
                                r.size(),
                                clicked_label,
                            ));
                        }
                        {
                            if let Some(item) = items.get(i).cloned() {
                                self.bump_counter_for(&item);
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
                                    ItemKind::PushTerminalCategories => {
                                        self.push_terminal_categories();
                                    }
                                    ItemKind::PushTerminalsInCategory(cid) => {
                                        self.push_terminals_in_category(cid);
                                    }
                                    ItemKind::OpenTextEditor(tid) => {
                                        self.open_edit_text_overlay(tid);
                                    }
                                    ItemKind::ViewText(tid) => {
                                        self.open_view_text_overlay(tid);
                                    }
                                    ItemKind::PushProjects => {
                                        self.push_projects();
                                    }
                                    ItemKind::OpenProjectTerminal(id) => {
                                        self.open_project_terminal(id);
                                    }
                                    ItemKind::EditProject(id) => {
                                        self.edit_project_overlay(id);
                                    }
                                    ItemKind::NewPlus => {
                                        self.editor_overlay = Some(EditorOverlay::NewPlus);
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
                    if self.shortcut_picker.is_some() {
                        self.draw_shortcut_picker(ui);
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
                    match final_nav_view_or_edit_modals::edit_single_text::ui::edit_single_text_ui(
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
                    match json::ui::json_ui(ui, &self.db, &mut self.json_state) {
                        json::ui::JsonAction::Back => self.page = Page::CircleMenu,
                        json::ui::JsonAction::Do => {}
                        json::ui::JsonAction::ImportConfirmed(payloads, mode) => {
                            let (text_count, tpl_count, _cat_count) = unwrap_or_bail!(
                                zutil_db::helpers::import::import_payloads(&self.db, payloads, mode)
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
                    match final_nav_view_or_edit_modals::edit_single_template::ui::edit_single_template_ui(
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
                    match final_nav_view_or_edit_modals::fill_template::ui::fill_template_ui(
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
