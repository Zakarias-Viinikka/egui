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

pub const TABLE_NAME: &str = "projects";

pub fn new_table_projects() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Text, TITLE.name),
        not_null_col(ColumnType::Text, PATH.name),
        int_col(LAUNCH_ZED.name, "0"),
        int_col(LAUNCH_ADSTUD.name, "0"),
        not_null_int_col(POPULARITY_CTR.name, "0"),
    ]
}

pub const TITLE: SchemaColumn<String> = SchemaColumn::<String> { name: "title", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const PATH: SchemaColumn<String> = SchemaColumn::<String> { name: "path", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const LAUNCH_ZED: SchemaColumn<i64> = SchemaColumn::<i64> { name: "launch_zed", type_of_col: &TypeOfCol::Integer, can_be_null: true, _marker: PhantomData };
pub const LAUNCH_ADSTUD: SchemaColumn<i64> = SchemaColumn::<i64> { name: "launch_adstud", type_of_col: &TypeOfCol::Integer, can_be_null: true, _marker: PhantomData };
pub const POPULARITY_CTR: SchemaColumn<i64> = SchemaColumn::<i64> { name: "popularity_ctr", type_of_col: &TypeOfCol::Integer, can_be_null: false, _marker: PhantomData };
