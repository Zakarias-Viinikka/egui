use protocol::payload::{ColumnValue, InsertDataIn};
use protocol::row_col::Col;

use crate::helpers::category::{TYPE_META, TYPE_NORMAL, category_id_opt};
use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;

pub fn new_row_text(title: String, body: String) -> InsertDataIn {
    new_row_text_with_category(title, body, None, None, None, false)
}

pub fn new_row_text_with_category(
    title: String,
    body: String,
    category_id: Option<i64>,
    meta_category_id: Option<i64>,
    type_of_text: Option<String>,
    copy_instead_of_view: bool,
) -> InsertDataIn {
    let category_value = match category_id {
        Some(id) => Col::Integer(id),
        None => Col::Null,
    };
    let meta_value = match meta_category_id {
        Some(id) => Col::Integer(id),
        None => Col::Null,
    };
    let type_value = match type_of_text {
        Some(s) if !s.is_empty() => Col::Text(s),
        _ => Col::Null,
    };

    InsertDataIn {
        table_name: "texts".to_string(),
        values: vec![
            ColumnValue {
                column_name: "title".to_string(),
                value: Col::Text(title),
            },
            ColumnValue {
                column_name: "body".to_string(),
                value: Col::Text(body),
            },
            ColumnValue {
                column_name: "category_id".to_string(),
                value: category_value,
            },
            ColumnValue {
                column_name: "meta_category_id".to_string(),
                value: meta_value,
            },
            ColumnValue {
                column_name: "type_of_text".to_string(),
                value: type_value,
            },
            ColumnValue {
                column_name: "copy_instead_of_view".to_string(),
                value: Col::Integer(if copy_instead_of_view { 1 } else { 0 }),
            },
        ],
    }
}

pub fn new_row_template(
    title: String,
    content: String,
    instructions: String,
    example: String,
    category_id: Option<i64>,
    meta_category_id: Option<i64>,
) -> InsertDataIn {
    let category_value = match category_id {
        Some(id) => Col::Integer(id),
        None => Col::Null,
    };
    let meta_value = match meta_category_id {
        Some(id) => Col::Integer(id),
        None => Col::Null,
    };

    InsertDataIn {
        table_name: "templates".to_string(),
        values: vec![
            ColumnValue {
                column_name: "title".to_string(),
                value: Col::Text(title),
            },
            ColumnValue {
                column_name: "content".to_string(),
                value: Col::Text(content),
            },
            ColumnValue {
                column_name: "instructions".to_string(),
                value: Col::Text(instructions),
            },
            ColumnValue {
                column_name: "example".to_string(),
                value: Col::Text(example),
            },
            ColumnValue {
                column_name: "category_id".to_string(),
                value: category_value,
            },
            ColumnValue {
                column_name: "meta_category_id".to_string(),
                value: meta_value,
            },
        ],
    }
}

pub fn new_row_keyword_lookup(text_id: i64, keyword: String) -> InsertDataIn {
    InsertDataIn {
        table_name: "keyword_lookup".to_string(),
        values: vec![
            ColumnValue {
                column_name: "text_id".to_string(),
                value: Col::Integer(text_id),
            },
            ColumnValue {
                column_name: "keyword".to_string(),
                value: Col::Text(keyword),
            },
        ],
    }
}

pub fn create_text_from_new_text(
    db: &LiveForever,
    nt: json_parsing::new_text::NewText,
) -> Result<InsertDataIn, DbError> {
    let category_id = match &nt.category {
        Some(name) => category_id_opt(db, name, TYPE_NORMAL)?,
        None => None,
    };
    let meta_category_id = match &nt.meta_category {
        Some(name) => category_id_opt(db, name, TYPE_META)?,
        None => None,
    };
    Ok(new_row_text_with_category(
        nt.title,
        nt.body,
        category_id,
        meta_category_id,
        nt.type_of_text,
        nt.copy_instead_of_view.unwrap_or(false),
    ))
}


pub fn create_template_from_new_template(
    db: &LiveForever,
    nt: json_parsing::new_template::NewTemplate,
) -> Result<InsertDataIn, DbError> {
    let category_id = match &nt.category {
        Some(name) => category_id_opt(db, name, TYPE_NORMAL)?,
        None => None,
    };
    let meta_category_id = match &nt.meta_category {
        Some(name) => category_id_opt(db, name, TYPE_META)?,
        None => None,
    };
    Ok(new_row_template(
        nt.title,
        nt.content,
        nt.instructions.unwrap_or_default(),
        nt.example.unwrap_or_default(),
        category_id,
        meta_category_id,
    ))
}
