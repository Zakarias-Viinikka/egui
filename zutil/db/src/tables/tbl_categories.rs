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

pub const TABLE_NAME: &str = "categories";

pub fn new_table_categories() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_unique_col(ColumnType::Text, NAME.name),
        not_null_text_col(TYPE_OF_CATEGORY.name, "normal"),
        not_null_int_col(POPULARITY_CTR.name, "0"),
    ]
}

pub const NAME: SchemaColumn<String> = SchemaColumn::<String> { name: "name", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const TYPE_OF_CATEGORY: SchemaColumn<String> = SchemaColumn::<String> { name: "type_of_category", type_of_col: &TypeOfCol::Text, can_be_null: false, _marker: PhantomData };
pub const POPULARITY_CTR: SchemaColumn<i64> = SchemaColumn::<i64> { name: "popularity_ctr", type_of_col: &TypeOfCol::Integer, can_be_null: false, _marker: PhantomData };
