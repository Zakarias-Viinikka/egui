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

pub const TABLE_NAME: &str = "shortcuts";

pub fn new_table_shortcuts() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Text, OWNER_KIND.name),
        not_null_col(ColumnType::Text, OWNER_ID.name),
        not_null_col(ColumnType::Text, COMBO.name),
    ]
}

pub const OWNER_KIND: SchemaColumn<String> = SchemaColumn::<String> { name: "owner_kind", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const OWNER_ID: SchemaColumn<String> = SchemaColumn::<String> { name: "owner_id", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const COMBO: SchemaColumn<String> = SchemaColumn::<String> { name: "combo", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
