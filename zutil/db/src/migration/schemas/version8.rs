use crate::migration::helpers::ADbTable;
use crate::migration::schemas::version7;
use crate::tables::tbl_starred_folders;
use db_wrapper::mascot::LiveForever;
use protocol::payload::CreateTableIn;

pub fn entire_table() -> Vec<ADbTable> {
    let mut tables = version7::entire_table();
    tables.push(ADbTable {
        table_name: tbl_starred_folders::TABLE_NAME.to_string(),
        columns: tbl_starred_folders::new_table_starred_folders(),
        foreign_keys: Vec::new(),
    });
    tables
}

pub fn update_from_old_version(db_wrapper: &LiveForever) {
    db_wrapper
        .create_table(CreateTableIn {
            table_name: tbl_starred_folders::TABLE_NAME.to_string(),
            columns: tbl_starred_folders::new_table_starred_folders(),
        })
        .unwrap();
}
