#![allow(dead_code)]

use db_wrapper::mascot::LiveForever;
use protocol::payload::*;
use std::fs;
use std::path::Path;

pub fn setup_fresh_db(name: &str) -> LiveForever {
    let path = Path::new("tests/test_db").join(name);
    fs::create_dir_all("tests/test_db").unwrap();
    let _ = fs::remove_file(&path);
    LiveForever::new(path.to_str().unwrap()).unwrap()
}

pub struct CheckTable<'a> {
    pub table_name: &'a str,
    pub db: &'a LiveForever,
}

pub fn check_table(params: CheckTable) -> Vec<TableColumnInfo> {
    params
        .db
        .check_table(CheckTableIn {
            table_name: params.table_name.to_string(),
        })
        .unwrap()
        .columns
}
