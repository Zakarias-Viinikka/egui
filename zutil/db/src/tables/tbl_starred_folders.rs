#![allow(non_snake_case)]
use std::marker::PhantomData;
use protocol::new_table::{ColumnDef, ColumnType, id_column, not_null_unique_col};
use protocol::schema_helper::{SchemaColumn, TypeOfCol};

pub const TABLE_NAME: &str = "starred_folders";

pub fn new_table_starred_folders() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_unique_col(ColumnType::Text, PATH.name),
    ]
}

pub const PATH: SchemaColumn<String> = SchemaColumn::<String> {
    name: "path",
    type_of_col: &TypeOfCol::Text,
    can_be_null: false,
    _marker: PhantomData,
};
