use crate::migration::helpers::ADbTable;
use crate::migration::schemas::version3;
use db_wrapper::mascot::LiveForever;
use protocol::new_table::{ColumnDef, ColumnType, id_column, not_null_unique_col};
use protocol::payload::{AddColumnIn, CreateTableIn};

pub fn entire_table() -> Vec<ADbTable> {
    let mut tables = version3::entire_table();
    for t in tables.iter_mut() {
        if matches!(
            t.table_name.as_str(),
            "texts" | "templates" | "projects" | "categories"
        ) {
            t.columns.push(popularity_column());
        }
    }
    tables.push(ADbTable {
        table_name: "main_nav_clicks".to_string(),
        columns: main_nav_clicks_columns(),
        foreign_keys: Vec::new(),
    });
    tables
}

pub fn popularity_column() -> ColumnDef {
    ColumnDef {
        name: "popularity_ctr".to_string(),
        column_type: "INTEGER".to_string(),
        primary_key: false,
        not_null: true,
        unique: false,
        default_value: "0".to_string(),
        autoincrement: false,
    }
}

pub fn main_nav_clicks_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_unique_col(ColumnType::Text, "name"),
        popularity_column(),
    ]
}

pub fn update_from_old_version(db_wrapper: &LiveForever) {
    for table in ["texts", "templates", "projects", "categories"] {
        db_wrapper
            .add_column(AddColumnIn {
                table_name: table.to_string(),
                column: popularity_column(),
            })
            .unwrap();
    }
    db_wrapper
        .create_table(CreateTableIn {
            table_name: "main_nav_clicks".to_string(),
            columns: main_nav_clicks_columns(),
        })
        .unwrap();
}
