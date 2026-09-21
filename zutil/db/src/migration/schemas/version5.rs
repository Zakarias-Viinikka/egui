use crate::migration::helpers::ADbTable;
use crate::migration::schemas::version4;
use db_wrapper::mascot::LiveForever;
use protocol::new_table::{ColumnType, id_column, not_null_col};
use protocol::payload::CreateTableIn;

pub fn entire_table() -> Vec<ADbTable> {
    let mut tables = version4::entire_table();
    tables.push(ADbTable {
        table_name: "shortcuts".to_string(),
        columns: shortcuts_columns(),
        foreign_keys: Vec::new(),
    });
    tables
}

pub fn shortcuts_columns() -> Vec<protocol::new_table::ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Text, "owner_kind"),
        not_null_col(ColumnType::Text, "owner_id"),
        not_null_col(ColumnType::Text, "combo"),
    ]
}

pub fn update_from_old_version(db_wrapper: &LiveForever) {
    db_wrapper
        .create_table(CreateTableIn {
            table_name: "shortcuts".to_string(),
            columns: shortcuts_columns(),
        })
        .unwrap();
}
