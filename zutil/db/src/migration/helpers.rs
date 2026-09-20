use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::new_table::{ColumnDef, ForeignKeyDef};
use protocol::payload::*;

#[derive(Clone)]
pub struct ADbTable {
    pub table_name: String,
    pub columns: Vec<ColumnDef>,
    pub foreign_keys: Vec<ForeignKeyDef>,
}

pub fn new_db_wrapper(path: &str) -> Result<LiveForever, DbError> {
    LiveForever::new(path)
}

pub fn create_table_from_scratch(
    db_tables: Vec<ADbTable>,
    db_wrapper: &LiveForever,
) -> Result<(), DbError> {
    for db_table in db_tables {
        if db_table.foreign_keys.is_empty() {
            db_wrapper.create_table(CreateTableIn {
                table_name: db_table.table_name,
                columns: db_table.columns,
            })?;
        } else {
            db_wrapper.create_foreign_table(CreateForeignTableIn {
                table_name: db_table.table_name,
                columns: db_table.columns,
                foreign_keys: db_table.foreign_keys,
            })?;
        }
    }
    Ok(())
}

pub fn drop_all_tables(db_tables: &[ADbTable], db_wrapper: &LiveForever) {
    for t in db_tables {
        db_wrapper
            .force_drop_table(DropTableIn {
                table_name: t.table_name.clone(),
            })
            .unwrap();
    }
}

pub fn table_shapes(db_tables: &[ADbTable], db_wrapper: &LiveForever) -> Vec<CheckTableOut> {
    db_tables
        .iter()
        .map(|t| {
            db_wrapper
                .check_table(CheckTableIn {
                    table_name: t.table_name.clone(),
                })
                .unwrap()
        })
        .collect()
}

pub fn assert_migrated_db_matches_fresh_db(db_tables: Vec<ADbTable>, db_wrapper: &LiveForever) {
    let shapes_before = table_shapes(&db_tables, db_wrapper);

    drop_all_tables(&db_tables, db_wrapper);
    create_table_from_scratch(db_tables.clone(), db_wrapper).unwrap();

    let shapes_after = table_shapes(&db_tables, db_wrapper);

    for (t, (before, after)) in db_tables
        .iter()
        .zip(shapes_before.iter().zip(shapes_after.iter()))
    {
        println!("checking table: {}", t.table_name);
        assert_eq!(before, after, "mismatch on table: {}", t.table_name);
    }
}
