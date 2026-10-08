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

pub const TABLE_NAME: &str = "template_fills";

pub fn new_table_template_fills() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Integer, TEMPLATE_ID.name),
        nullable_col(LABEL.name, "TEXT"),
        not_null_col(ColumnType::Text, VALUES_JSON.name),
    ]
}

pub fn foreign_keys() -> Vec<ForeignKeyDef> {
    vec![ForeignKeyDef { column_name: TEMPLATE_ID.name.to_string(), referenced_table_name: "templates".to_string(), referenced_column_name: "id".to_string() }]
}

pub const TEMPLATE_ID: SchemaColumn<i64> = SchemaColumn::<i64> { name: "template_id", type_of_col: &TypeOfCol::Integer, can_be_null: false, _marker: PhantomData };
pub const LABEL: SchemaColumn<String> = SchemaColumn::<String> { name: "label", type_of_col: &TypeOfCol::Text, can_be_null: true, _marker: PhantomData };
pub const VALUES_JSON: SchemaColumn<String> = SchemaColumn::<String> { name: "values_json", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
