use crate::migration::helpers::ADbTable;
use crate::migration::schemas::version2;
use db_wrapper::mascot::LiveForever;
use protocol::new_table::ColumnDef;
use protocol::payload::AddColumnIn;

pub fn entire_table() -> Vec<ADbTable> {
    let mut tables = version2::entire_table();
    for t in tables.iter_mut() {
        if t.table_name == "projects" {
            t.columns.push(nullable_int("launch_zed"));
            t.columns.push(nullable_int("launch_adstud"));
        }
    }
    tables
}

fn nullable_int(name: &str) -> ColumnDef {
    ColumnDef {
        name: name.to_string(),
        column_type: "INTEGER".to_string(),
        primary_key: false,
        not_null: false,
        unique: false,
        default_value: "0".to_string(),
        autoincrement: false,
    }
}

pub fn update_from_old_version(db_wrapper: &LiveForever) {
    for name in ["launch_zed", "launch_adstud"] {
        db_wrapper
            .add_column(AddColumnIn {
                table_name: "projects".to_string(),
                column: nullable_int(name),
            })
            .unwrap();
    }
}
