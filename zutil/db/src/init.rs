use crate::migration::helpers::{create_table_from_scratch, new_db_wrapper};
use crate::migration::schema_versions::{CURRENT_VERSION, SchemaVersion};
use crate::migration::schemas::current;
use crate::migration::update_from_old_schema::update_until_newest_version;
use db_wrapper::mascot::LiveForever;
use protocol::new_table::{ColumnType, id_column, not_null_col};
use protocol::payload::{
    ColumnValue, CreateFts5TableIn, CreateTableIn, EditColInRowIn, GetDataIn, InsertDataIn,
    SelectArgument, SelectArguments,
};
use protocol::row_col::Col;

pub fn init_db() -> LiveForever {
    let db = new_db_wrapper(&default_db_path()).unwrap();

    ensure_version_table(&db);

    let is_fresh = !db
        .list_tables()
        .unwrap()
        .table_names
        .contains(&"texts".to_string());

    if is_fresh {
        create_table_from_scratch(current::entire_table(), &db).unwrap();
        write_version(&db, CURRENT_VERSION.to_int());
    } else {
        let stored = read_version(&db);
        let current = stored
            .and_then(SchemaVersion::from_int)
            .unwrap_or(SchemaVersion::Version0);

        if current != CURRENT_VERSION {
            let new_v = update_until_newest_version(current, &db);
            write_version(&db, new_v.to_int());
        }
    }

    setup_fts5(&db);

    db
}

fn ensure_version_table(db: &LiveForever) {
    if db
        .list_tables()
        .unwrap()
        .table_names
        .contains(&"schema_version".to_string())
    {
        return;
    }

    db.create_table(CreateTableIn {
        table_name: "schema_version".to_string(),
        columns: vec![
            id_column(),
            not_null_col(ColumnType::Integer, "version"),
        ],
    })
    .unwrap();
}

fn read_version(db: &LiveForever) -> Option<i64> {
    let out = db
        .get_data(GetDataIn {
            table_name: "schema_version".to_string(),
            arguments: SelectArguments::Single(SelectArgument::All),
            columns_to_read: vec!["version".to_string()],
        })
        .ok()?;

    let row = out.rows.into_iter().next()?;
    let col = row.cols.into_iter().next()?;
    col.as_int().ok().copied()
}

fn write_version(db: &LiveForever, v: i64) {
    if read_version(db).is_some() {
        db.edit_col_in_row(EditColInRowIn {
            table_name: "schema_version".to_string(),
            row_id: "1".to_string(),
            column: "version".to_string(),
            new_value: Col::Integer(v),
        })
        .unwrap();
    } else {
        db.insert_data(InsertDataIn {
            table_name: "schema_version".to_string(),
            values: vec![ColumnValue {
                column_name: "version".to_string(),
                value: Col::Integer(v),
            }],
        })
        .unwrap();
    }
}

fn default_db_path() -> String {
    let home = std::env::var("HOME").expect("HOME not set");
    let dir = std::path::Path::new(&home).join(".local/share/zutil");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("zutil.sqlite")
        .to_str()
        .unwrap()
        .to_string()
}

pub fn setup_fts5(db: &LiveForever) {
    let tables = db.list_tables().unwrap().table_names;
    if tables.iter().any(|t| t.starts_with("fts5_")) {
        return;
    }
    db.create_fts5_table(CreateFts5TableIn {
        source_table_name: "keyword_lookup".to_string(),
        columns: vec!["keyword".to_string()],
    })
    .unwrap();
}
