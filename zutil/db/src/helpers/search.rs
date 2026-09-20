use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{GetDataIn, SelectArgument, SelectArguments};
use protocol::row_col::Row;

use crate::helpers::read::{read_all_keyword_lookups, read_all_templates, read_all_texts};

pub fn search_texts_by_title(db: &LiveForever, query: &str) -> Result<Vec<Row>, DbError> {
    if query.is_empty() {
        return read_all_texts(db);
    }

    Ok(db
        .get_data(GetDataIn {
            table_name: "texts".to_string(),
            arguments: SelectArguments::Single(SelectArgument::XLikeY {
                x: "title".to_string(),
                y: format!("%{}%", query),
            }),
            columns_to_read: Vec::new(),
        })?
        .rows)
}

pub fn search_keyword_lookups_by_keyword(
    db: &LiveForever,
    query: &str,
) -> Result<Vec<Row>, DbError> {
    if query.is_empty() {
        return read_all_keyword_lookups(db);
    }

    Ok(db
        .get_data(GetDataIn {
            table_name: "keyword_lookup".to_string(),
            arguments: SelectArguments::Single(SelectArgument::XLikeY {
                x: "keyword".to_string(),
                y: format!("%{}%", query),
            }),
            columns_to_read: Vec::new(),
        })?
        .rows)
}

pub fn search_templates_by_title(db: &LiveForever, query: &str) -> Result<Vec<Row>, DbError> {
    if query.is_empty() {
        return read_all_templates(db);
    }

    Ok(db
        .get_data(GetDataIn {
            table_name: "templates".to_string(),
            arguments: SelectArguments::Single(SelectArgument::XLikeY {
                x: "title".to_string(),
                y: format!("%{}%", query),
            }),
            columns_to_read: Vec::new(),
        })?
        .rows)
}
