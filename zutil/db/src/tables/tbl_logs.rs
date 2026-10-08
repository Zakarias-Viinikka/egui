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

pub const TABLE_NAME: &str = "logs";

pub fn new_table_logs() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Integer, TIMESTAMP.name),
        not_null_col(ColumnType::Text, LEVEL.name),
        not_null_col(ColumnType::Text, CATEGORY.name),
        not_null_col(ColumnType::Text, SOURCE.name),
        not_null_col(ColumnType::Text, SESSION_ID.name),
        not_null_col(ColumnType::Text, MESSAGE.name),
        nullable_col(DETAILS.name, "BLOB"),
        nullable_col(DETAILS_TYPE.name, "TEXT"),
    ]
}

pub const TIMESTAMP: SchemaColumn<i64> = SchemaColumn::<i64> { name: "timestamp", type_of_col: &TypeOfCol::Integer, can_be_null: false, _marker: PhantomData };
pub const LEVEL: SchemaColumn<String> = SchemaColumn::<String> { name: "level", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const CATEGORY: SchemaColumn<String> = SchemaColumn::<String> { name: "category", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const SOURCE: SchemaColumn<String> = SchemaColumn::<String> { name: "source", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const SESSION_ID: SchemaColumn<String> = SchemaColumn::<String> { name: "session_id", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const MESSAGE: SchemaColumn<String> = SchemaColumn::<String> { name: "message", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const DETAILS: SchemaColumn<Vec<u8>> = SchemaColumn::<Vec<u8>> { name: "details", type_of_col: &TypeOfCol::Blob, can_be_null: true, _marker: PhantomData };
pub const DETAILS_TYPE: SchemaColumn<String> = SchemaColumn::<String> { name: "details_type", type_of_col: &TypeOfCol::Text, can_be_null: true, _marker: PhantomData };
