use db_wrapper::mascot::LiveForever;
use eframe::egui;
use error_stuff::unwrap_or_bail;
use std::collections::HashMap;
use zutil_db::helpers::category::read_all_categories_with_ids;
use zutil_db::helpers::read::{read_all_templates, read_all_texts};

#[derive(Clone, Copy, PartialEq)]
pub enum RowKind {
    Text,
    Template,
}

pub struct ListRow {
    pub kind: RowKind,
    pub id: i64,
    pub title: String,
    pub type_of_text: String,
    pub category: String,
    pub meta_category: String,
}

pub struct EditTextState {
    pub search_query: String,
    pub rows: Vec<ListRow>,
}

impl Default for EditTextState {
    fn default() -> Self {
        Self { search_query: String::new(), rows: Vec::new() }
    }
}

impl EditTextState {
    pub fn reload(&mut self, db: &LiveForever) {
        let rows = unwrap_or_bail!(
            load_rows(db, &self.search_query),
            "edit_text",
            "reload"
        );
        self.rows = rows;
    }
}

pub enum EditTextAction {
    None,
    Back,
    NewText,
    NewTemplate,
    EditText(i64),
    EditTemplate(i64),
    FillTemplate(i64),
    DeleteText(i64),
    DeleteTemplate(i64),
}

pub fn edit_text_ui(
    ui: &mut egui::Ui,
    db: &LiveForever,
    state: &mut EditTextState,
) -> EditTextAction {
    let mut action = EditTextAction::None;

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            action = EditTextAction::Back;
        }
        if ui.text_edit_singleline(&mut state.search_query).changed() {
            state.reload(db);
        }
    });

    ui.separator();

    const BOTTOM_HEIGHT: f32 = 110.0;
    let avail = ui.available_size();
    let scroll_height = (avail.y - BOTTOM_HEIGHT).max(0.0);

    egui::ScrollArea::vertical()
        .max_height(scroll_height)
        .show(ui, |ui| {
            for row in &state.rows {
                ui.horizontal(|ui| {
                    match row.kind {
                        RowKind::Text => {
                            ui.weak("[text]");
                        }
                        RowKind::Template => {
                            ui.weak("[tpl]");
                        }
                    }
                    ui.label(&row.title);
                    if !row.type_of_text.is_empty() {
                        ui.weak(format!("<{}>", row.type_of_text));
                    }
                    if !row.category.is_empty() {
                        ui.weak(format!("[{}]", row.category));
                    }
                    if !row.meta_category.is_empty() {
                        ui.weak(format!("({})", row.meta_category));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Delete").clicked() {
                            action = match row.kind {
                                RowKind::Text => EditTextAction::DeleteText(row.id),
                                RowKind::Template => EditTextAction::DeleteTemplate(row.id),
                            };
                        }
                        if row.kind == RowKind::Template && ui.button("Fill").clicked() {
                            action = EditTextAction::FillTemplate(row.id);
                        }
                        if ui.button("Edit").clicked() {
                            action = match row.kind {
                                RowKind::Text => EditTextAction::EditText(row.id),
                                RowKind::Template => EditTextAction::EditTemplate(row.id),
                            };
                        }
                    });
                });
            }
        });

    ui.add_space(8.0);
    let width = ui.available_width();
    if ui.add_sized([width, 40.0], egui::Button::new("+ Text")).clicked() {
        action = EditTextAction::NewText;
    }
    if ui.add_sized([width, 40.0], egui::Button::new("+ Template")).clicked() {
        action = EditTextAction::NewTemplate;
    }

    action
}

fn load_rows(db: &LiveForever, search: &str) -> Result<Vec<ListRow>, String> {
    let text_rows = read_all_texts(db).map_err(|e| e.to_string())?;
    let tpl_rows = read_all_templates(db).map_err(|e| e.to_string())?;
    let triples = read_all_categories_with_ids(db).map_err(|e| e.to_string())?;
    let name_by_id: HashMap<i64, String> = triples
        .into_iter()
        .map(|(id, name, _)| (id, name))
        .collect();

    let needle = search.trim().to_lowercase();

    let mut out = Vec::new();
    for r in text_rows {
        let row = parse_row(r, RowKind::Text, &name_by_id)?;
        if matches_search(&row, &needle) {
            out.push(row);
        }
    }
    for r in tpl_rows {
        let row = parse_row(r, RowKind::Template, &name_by_id)?;
        if matches_search(&row, &needle) {
            out.push(row);
        }
    }
    Ok(out)
}

fn matches_search(row: &ListRow, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    row.title.to_lowercase().contains(needle)
        || row.category.to_lowercase().contains(needle)
        || row.meta_category.to_lowercase().contains(needle)
        || row.type_of_text.to_lowercase().contains(needle)
}

fn parse_row(
    r: protocol::row_col::Row,
    kind: RowKind,
    name_by_id: &HashMap<i64, String>,
) -> Result<ListRow, String> {
    let id = *r.cols[0].as_int().map_err(|e| e)?;
    let title = r.cols[1].as_str().map_err(|e| e)?.to_string();
    let (cat_idx, meta_idx, type_idx) = match kind {
        RowKind::Text => (3, 4, Some(5)),
        RowKind::Template => (5, 6, None),
    };
    let category = r
        .cols
        .get(cat_idx)
        .and_then(|c| c.as_int().ok())
        .and_then(|cid| name_by_id.get(cid).cloned())
        .unwrap_or_default();
    let meta_category = r
        .cols
        .get(meta_idx)
        .and_then(|c| c.as_int().ok())
        .and_then(|cid| name_by_id.get(cid).cloned())
        .unwrap_or_default();
    let type_of_text = type_idx
        .and_then(|i| r.cols.get(i))
        .and_then(|c| c.as_str().ok())
        .unwrap_or("")
        .to_string();
    Ok(ListRow {
        kind,
        id,
        title,
        type_of_text,
        category,
        meta_category,
    })
}
