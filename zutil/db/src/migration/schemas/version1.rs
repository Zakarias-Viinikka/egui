use crate::migration::helpers::ADbTable;
use crate::migration::schemas::version0;
use db_wrapper::mascot::LiveForever;
use protocol::new_table::ColumnDef;
use protocol::payload::AddColumnIn;

pub fn entire_table() -> Vec<ADbTable> {
    let mut tables = version0::entire_table();
    for t in tables.iter_mut() {
        if t.table_name == "categories" {
            t.columns.push(type_of_category_column());
        }
    }
    tables
}

pub fn type_of_category_column() -> ColumnDef {
    ColumnDef {
        name: "type_of_category".to_string(),
        column_type: "TEXT".to_string(),
        primary_key: false,
        not_null: true,
        unique: false,
        default_value: "normal".to_string(),
        autoincrement: false,
    }
}

pub fn update_from_old_version(db_wrapper: &LiveForever) {
    db_wrapper
        .add_column(AddColumnIn {
            table_name: "categories".to_string(),
            column: type_of_category_column(),
        })
        .unwrap();
}
