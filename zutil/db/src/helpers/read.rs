use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{GetDataIn, SelectArgument, SelectArguments};
use protocol::row_col::Row;

pub fn read_all_texts(db: &LiveForever) -> Result<Vec<Row>, DbError> {
    read_all(db, "texts")
}

pub fn read_all_keyword_lookups(db: &LiveForever) -> Result<Vec<Row>, DbError> {
    read_all(db, "keyword_lookup")
}

pub fn read_texts_by_ids(db: &LiveForever, ids: Vec<i64>) -> Result<Vec<Row>, DbError> {
    read_by_ids(db, "texts", ids)
}

pub fn read_keyword_lookups_by_ids(db: &LiveForever, ids: Vec<i64>) -> Result<Vec<Row>, DbError> {
    read_by_ids(db, "keyword_lookup", ids)
}

pub fn read_text_by_id(db: &LiveForever, id: i64) -> Result<Option<Row>, DbError> {
    Ok(read_by_ids(db, "texts", vec![id])?.into_iter().next())
}

pub fn read_all_templates(db: &LiveForever) -> Result<Vec<Row>, DbError> {
    read_all(db, "templates")
}

pub fn read_template_by_id(db: &LiveForever, id: i64) -> Result<Option<Row>, DbError> {
    Ok(read_by_ids(db, "templates", vec![id])?.into_iter().next())
}

pub fn read_keyword_lookup_by_id(db: &LiveForever, id: i64) -> Result<Option<Row>, DbError> {
    Ok(read_by_ids(db, "keyword_lookup", vec![id])?.into_iter().next())
}

fn read_all(db: &LiveForever, table_name: &str) -> Result<Vec<Row>, DbError> {
    Ok(db
        .get_data(GetDataIn {
            table_name: table_name.to_string(),
            arguments: SelectArguments::Single(SelectArgument::All),
            columns_to_read: Vec::new(),
        })?
        .rows)
}

fn read_by_ids(db: &LiveForever, table_name: &str, ids: Vec<i64>) -> Result<Vec<Row>, DbError> {
    let id_strings: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    Ok(db
        .get_data(GetDataIn {
            table_name: table_name.to_string(),
            arguments: SelectArguments::Single(SelectArgument::XInY {
                x: "id".to_string(),
                y: id_strings,
            }),
            columns_to_read: Vec::new(),
        })?
        .rows)
}
