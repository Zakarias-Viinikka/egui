#![allow(non_snake_case)]
use std::marker::PhantomData;
use protocol::new_table::{ColumnDef, ColumnType, ForeignKeyDef, id_column, not_null_col, not_null_unique_col};
use protocol::schema_helper::{SchemaColumn, TypeOfCol};

fn nullable_col(name: &str, ty: &str) -> ColumnDef {
    ColumnDef {
        name: name.to_string(),
        column_type: ty.to_string(),
        primary_key: false,
        not_null: false,
        unique: false,
        default_value: "".to_string(),
        autoincrement: false,
    }
}

fn int_col(name: &str, default: &str) -> ColumnDef {
    ColumnDef {
        name: name.to_string(),
        column_type: "INTEGER".to_string(),
        primary_key: false,
        not_null: false,
        unique: false,
        default_value: default.to_string(),
        autoincrement: false,
    }
}

fn not_null_int_col(name: &str, default: &str) -> ColumnDef {
    ColumnDef {
        name: name.to_string(),
        column_type: "INTEGER".to_string(),
        primary_key: false,
        not_null: true,
        unique: false,
        default_value: default.to_string(),
        autoincrement: false,
    }
}

fn not_null_text_col(name: &str, default: &str) -> ColumnDef {
    ColumnDef {
        name: name.to_string(),
        column_type: "TEXT".to_string(),
        primary_key: false,
        not_null: true,
        unique: false,
        default_value: default.to_string(),
        autoincrement: false,
    }
}

pub const TABLE_NAME: &str = "error_log";

pub fn new_table_error_log() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Integer, TIMESTAMP.name),
        not_null_col(ColumnType::Text, SCREEN.name),
        not_null_col(ColumnType::Text, LOCATION.name),
        not_null_col(ColumnType::Text, DETAIL.name),
    ]
}

pub const TIMESTAMP: SchemaColumn<i64> = SchemaColumn::<i64> { name: "timestamp", type_of_col: &TypeOfCol::Integer, can_be_null: false, _marker: PhantomData };
pub const SCREEN: SchemaColumn<String> = SchemaColumn::<String> { name: "screen", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const LOCATION: SchemaColumn<String> = SchemaColumn::<String> { name: "location", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const DETAIL: SchemaColumn<String> = SchemaColumn::<String> { name: "detail", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
