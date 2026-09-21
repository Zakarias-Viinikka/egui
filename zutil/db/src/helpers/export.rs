use db_wrapper::mascot::LiveForever;
use json_parsing::new_category::NewCategory;
use json_parsing::new_template::NewTemplate;
use json_parsing::new_text::NewText;
use protocol::error::DbError;
use protocol::row_col::Row;
use std::collections::HashMap;

use crate::helpers::category::read_all_categories_with_ids;
use crate::helpers::read::{read_all_templates, read_all_texts};

pub fn gather_for_export(
    db: &LiveForever,
) -> Result<(Vec<NewCategory>, Vec<NewText>, Vec<NewTemplate>), DbError> {
    let triples = read_all_categories_with_ids(db)?;
    let name_by_id: HashMap<i64, String> = triples
        .iter()
        .map(|(id, name, _)| (*id, name.clone()))
        .collect();

    let mut categories = Vec::new();
    for (_, name, t) in &triples {
        categories.push(NewCategory {
            name: name.clone(),
            type_of_category: t.clone(),
        });
    }

    let mut texts = Vec::new();
    for r in read_all_texts(db)? {
        texts.push(row_to_new_text(&r, &name_by_id));
    }

    let mut templates = Vec::new();
    for r in read_all_templates(db)? {
        templates.push(row_to_new_template(&r, &name_by_id));
    }

    Ok((categories, texts, templates))
}

fn row_to_new_text(r: &Row, name_by_id: &HashMap<i64, String>) -> NewText {
    let title = r.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
    let body = r.cols.get(2).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
    let category = r
        .cols
        .get(3)
        .and_then(|c| c.as_int().ok())
        .and_then(|id| name_by_id.get(id).cloned());
    let meta_category = r
        .cols
        .get(4)
        .and_then(|c| c.as_int().ok())
        .and_then(|id| name_by_id.get(id).cloned());
    let type_of_text = r
        .cols
        .get(5)
        .and_then(|c| c.as_str().ok())
        .map(|s| s.to_string());
    let copy_instead_of_view = r
        .cols
        .get(6)
        .and_then(|c| c.as_int().ok().copied())
        .map(|v| v != 0)
        .unwrap_or(false);
    NewText {
        title,
        body,
        category,
        meta_category,
        type_of_text,
        copy_instead_of_view: if copy_instead_of_view { Some(true) } else { None },
    }
}

fn row_to_new_template(r: &Row, name_by_id: &HashMap<i64, String>) -> NewTemplate {
    let title = r.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
    let content = r.cols.get(2).and_then(|c| c.as_str().ok()).unwrap_or("").to_string();
    let instructions = r
        .cols
        .get(3)
        .and_then(|c| c.as_str().ok())
        .map(|s| s.to_string());
    let example = r
        .cols
        .get(4)
        .and_then(|c| c.as_str().ok())
        .map(|s| s.to_string());
    let category = r
        .cols
        .get(5)
        .and_then(|c| c.as_int().ok())
        .and_then(|id| name_by_id.get(id).cloned());
    let meta_category = r
        .cols
        .get(6)
        .and_then(|c| c.as_int().ok())
        .and_then(|id| name_by_id.get(id).cloned());
    NewTemplate {
        title,
        content,
        instructions,
        example,
        category,
        meta_category,
    }
}
