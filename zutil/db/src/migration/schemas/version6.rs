use crate::migration::helpers::ADbTable;
use crate::migration::schemas::version5;
use db_wrapper::mascot::LiveForever;
use protocol::new_table::{ColumnType, id_column, not_null_col};
use protocol::payload::CreateTableIn;

pub fn entire_table() -> Vec<ADbTable> {
    let mut tables = version5::entire_table();
    tables.push(ADbTable {
        table_name: "error_log".to_string(),
        columns: error_log_columns(),
        foreign_keys: Vec::new(),
    });
    tables
}

pub fn error_log_columns() -> Vec<protocol::new_table::ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Integer, "timestamp"),
        not_null_col(ColumnType::Text, "screen"),
        not_null_col(ColumnType::Text, "location"),
        not_null_col(ColumnType::Text, "detail"),
    ]
}

pub fn update_from_old_version(db_wrapper: &LiveForever) {
    // the shortcuts table was created in v5 with a UNIQUE on combo.
    // v6 rebuilds it without that constraint so different circles can
    // reuse the same key. Dropping is safe: v5 shipped yesterday and
    // holds at most a handful of rows.
    db_wrapper
        .force_drop_table(protocol::payload::DropTableIn {
            table_name: "shortcuts".to_string(),
        })
        .unwrap();
    db_wrapper
        .create_table(CreateTableIn {
            table_name: "shortcuts".to_string(),
            columns: crate::migration::schemas::version5::shortcuts_columns(),
        })
        .unwrap();

    db_wrapper
        .create_table(CreateTableIn {
            table_name: "error_log".to_string(),
            columns: error_log_columns(),
        })
        .unwrap();
}
