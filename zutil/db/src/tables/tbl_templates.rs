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

pub const TABLE_NAME: &str = "templates";

pub fn new_table_templates() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Text, TITLE.name),
        not_null_col(ColumnType::Text, CONTENT.name),
        nullable_col(INSTRUCTIONS.name, "TEXT"),
        nullable_col(EXAMPLE.name, "TEXT"),
        nullable_col(CATEGORY_ID.name, "INTEGER"),
        nullable_col(META_CATEGORY_ID.name, "INTEGER"),
        not_null_int_col(POPULARITY_CTR.name, "0"),
    ]
}

pub fn foreign_keys() -> Vec<ForeignKeyDef> {
    vec![
        ForeignKeyDef { column_name: CATEGORY_ID.name.to_string(), referenced_table_name: "categories".to_string(), referenced_column_name: "id".to_string() },
        ForeignKeyDef { column_name: META_CATEGORY_ID.name.to_string(), referenced_table_name: "categories".to_string(), referenced_column_name: "id".to_string() },
    ]
}

pub const TITLE: SchemaColumn<String> = SchemaColumn::<String> { name: "title", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const CONTENT: SchemaColumn<String> = SchemaColumn::<String> { name: "content", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const INSTRUCTIONS: SchemaColumn<String> = SchemaColumn::<String> { name: "instructions", type_of_col: &TypeOfCol::Text, can_be_null: true, _marker: PhantomData };
pub const EXAMPLE: SchemaColumn<String> = SchemaColumn::<String> { name: "example", type_of_col: &TypeOfCol::Text, can_be_null: true, _marker: PhantomData };
pub const CATEGORY_ID: SchemaColumn<i64> = SchemaColumn::<i64> { name: "category_id", type_of_col: &TypeOfCol::Integer, can_be_null: true, _marker: PhantomData };
pub const META_CATEGORY_ID: SchemaColumn<i64> = SchemaColumn::<i64> { name: "meta_category_id", type_of_col: &TypeOfCol::Integer, can_be_null: true, _marker: PhantomData };
pub const POPULARITY_CTR: SchemaColumn<i64> = SchemaColumn::<i64> { name: "popularity_ctr", type_of_col: &TypeOfCol::Integer, can_be_null: false, _marker: PhantomData };
