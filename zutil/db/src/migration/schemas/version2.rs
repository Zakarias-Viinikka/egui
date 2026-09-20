use crate::migration::helpers::ADbTable;
use crate::migration::schemas::version1;
use db_wrapper::mascot::LiveForever;
use protocol::new_table::{ColumnDef, ColumnType, id_column, not_null_col};
use protocol::payload::CreateTableIn;

pub fn entire_table() -> Vec<ADbTable> {
    let mut tables = version1::entire_table();
    tables.push(ADbTable {
        table_name: "projects".to_string(),
        columns: projects_columns(),
        foreign_keys: Vec::new(),
    });
    tables
}

pub fn projects_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Text, "title"),
        not_null_col(ColumnType::Text, "path"),
    ]
}

pub fn update_from_old_version(db_wrapper: &LiveForever) {
    db_wrapper
        .create_table(CreateTableIn {
            table_name: "projects".to_string(),
            columns: projects_columns(),
        })
        .unwrap();
}
