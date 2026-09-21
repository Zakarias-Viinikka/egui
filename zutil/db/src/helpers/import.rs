use db_wrapper::mascot::LiveForever;
use protocol::payload::{DeleteRowIn, GetDataIn, SelectArgument, SelectArguments};
use json_parsing::parse::ParsedPayload;
use protocol::error::DbError;

use crate::helpers::category::category_id_opt;
use crate::helpers::new_row::{
    create_template_from_new_template, create_text_from_new_text,
};

#[derive(Clone, Copy, PartialEq)]
pub enum ImportMode {
    KeepBoth,
    Replace,
}

pub fn title_exists(db: &LiveForever, table: &str, title: &str) -> Result<bool, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: table.to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "title".to_string(),
            y: title.to_string(),
        }),
        columns_to_read: vec!["id".to_string()],
    })?;
    Ok(!out.rows.is_empty())
}

fn delete_by_title(db: &LiveForever, table: &str, title: &str) -> Result<(), DbError> {
    let out = db.get_data(GetDataIn {
        table_name: table.to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "title".to_string(),
            y: title.to_string(),
        }),
        columns_to_read: vec!["id".to_string()],
    })?;
    for row in out.rows {
        if let Some(id) = row.cols.first().and_then(|c| c.as_int().ok().copied()) {
            db.delete_row(DeleteRowIn {
                table_name: table.to_string(),
                row_id: id.to_string(),
            })?;
        }
    }
    Ok(())
}

pub fn import_payloads(
    db: &LiveForever,
    payloads: Vec<ParsedPayload>,
    mode: ImportMode,
) -> Result<(usize, usize, usize), DbError> {
    db.begin_all_or_nothing()?;

    let mut text_count = 0usize;
    let mut tpl_count = 0usize;
    let mut cat_count = 0usize;

    let result = (|| -> Result<(), DbError> {
        for payload in payloads {
            match payload {
                ParsedPayload::NewTexts(items) => {
                    for nt in items {
                        if mode == ImportMode::Replace {
                            delete_by_title(db, "texts", &nt.title)?;
                        }
                        let row = create_text_from_new_text(db, nt)?;
                        db.insert_data(row)?;
                        text_count += 1;
                    }
                }
                ParsedPayload::NewTemplates(items) => {
                    for nt in items {
                        if mode == ImportMode::Replace {
                            delete_by_title(db, "templates", &nt.title)?;
                        }
                        let row = create_template_from_new_template(db, nt)?;
                        db.insert_data(row)?;
                        tpl_count += 1;
                    }
                }
                ParsedPayload::NewCategories(items) => {
                    for c in items {
                        if category_id_opt(db, &c.name, &c.type_of_category)?.is_some() {
                            cat_count += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            db.everything_went_perfectly()?;
            Ok((text_count, tpl_count, cat_count))
        }
        Err(e) => {
            let _ = db.regret_everything();
            Err(e)
        }
    }
}
