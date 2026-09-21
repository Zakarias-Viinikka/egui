use crate::migration::helpers::ADbTable;
use crate::migration::schemas::version6;
use db_wrapper::mascot::LiveForever;
use protocol::new_table::ColumnDef;
use protocol::payload::AddColumnIn;

pub fn entire_table() -> Vec<ADbTable> {
    let mut tables = version6::entire_table();
    for t in tables.iter_mut() {
        if t.table_name == "texts" {
            t.columns.push(copy_instead_of_view_column());
        }
    }
    tables
}

pub fn copy_instead_of_view_column() -> ColumnDef {
    ColumnDef {
        name: "copy_instead_of_view".to_string(),
        column_type: "INTEGER".to_string(),
        primary_key: false,
        not_null: true,
        unique: false,
        default_value: "0".to_string(),
        autoincrement: false,
    }
}

pub fn update_from_old_version(db_wrapper: &LiveForever) {
    db_wrapper
        .add_column(AddColumnIn {
            table_name: "texts".to_string(),
            column: copy_instead_of_view_column(),
        })
        .unwrap();
}
