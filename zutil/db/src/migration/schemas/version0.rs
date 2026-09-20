use crate::migration::helpers::ADbTable;
use protocol::new_table::{
    ColumnDef, ColumnType, ForeignKeyDef, id_column, not_null_col, not_null_unique_col,
};

pub fn entire_table() -> Vec<ADbTable> {
    vec![
        ADbTable {
            table_name: "categories".to_string(),
            columns: categories_columns(),
            foreign_keys: Vec::new(),
        },
        ADbTable {
            table_name: "texts".to_string(),
            columns: texts_columns(),
            foreign_keys: vec![
                ForeignKeyDef {
                    column: "category_id".to_string(),
                    referenced_table: "categories".to_string(),
                    referenced_column: "id".to_string(),
                },
                ForeignKeyDef {
                    column: "meta_category_id".to_string(),
                    referenced_table: "categories".to_string(),
                    referenced_column: "id".to_string(),
                },
            ],
        },
        ADbTable {
            table_name: "keyword_lookup".to_string(),
            columns: keyword_lookup_columns(),
            foreign_keys: vec![ForeignKeyDef {
                column: "text_id".to_string(),
                referenced_table: "texts".to_string(),
                referenced_column: "id".to_string(),
            }],
        },
        ADbTable {
            table_name: "logs".to_string(),
            columns: logs_columns(),
            foreign_keys: Vec::new(),
        },
        ADbTable {
            table_name: "key_value_storage".to_string(),
            columns: key_value_storage_columns(),
            foreign_keys: Vec::new(),
        },
        ADbTable {
            table_name: "templates".to_string(),
            columns: templates_columns(),
            foreign_keys: vec![
                ForeignKeyDef {
                    column: "category_id".to_string(),
                    referenced_table: "categories".to_string(),
                    referenced_column: "id".to_string(),
                },
                ForeignKeyDef {
                    column: "meta_category_id".to_string(),
                    referenced_table: "categories".to_string(),
                    referenced_column: "id".to_string(),
                },
            ],
        },
        ADbTable {
            table_name: "template_fills".to_string(),
            columns: template_fills_columns(),
            foreign_keys: vec![ForeignKeyDef {
                column: "template_id".to_string(),
                referenced_table: "templates".to_string(),
                referenced_column: "id".to_string(),
            }],
        },
    ]
}

pub fn categories_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_unique_col(ColumnType::Text, "name"),
    ]
}

pub fn texts_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Text, "title"),
        not_null_col(ColumnType::Text, "body"),
        nullable_text_or_int("category_id", "INTEGER"),
        nullable_text_or_int("meta_category_id", "INTEGER"),
        nullable_text_or_int("type_of_text", "TEXT"),
    ]
}

pub fn keyword_lookup_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Integer, "text_id"),
        not_null_col(ColumnType::Text, "keyword"),
    ]
}

pub fn logs_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Integer, "timestamp"),
        not_null_col(ColumnType::Text, "level"),
        not_null_col(ColumnType::Text, "category"),
        not_null_col(ColumnType::Text, "source"),
        not_null_col(ColumnType::Text, "session_id"),
        not_null_col(ColumnType::Text, "message"),
        nullable_text_or_int("details", "BLOB"),
        nullable_text_or_int("details_type", "TEXT"),
    ]
}

pub fn templates_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Text, "title"),
        not_null_col(ColumnType::Text, "content"),
        nullable_text_or_int("instructions", "TEXT"),
        nullable_text_or_int("example", "TEXT"),
        nullable_text_or_int("category_id", "INTEGER"),
        nullable_text_or_int("meta_category_id", "INTEGER"),
    ]
}

pub fn template_fills_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_col(ColumnType::Integer, "template_id"),
        nullable_text_or_int("label", "TEXT"),
        not_null_col(ColumnType::Text, "values_json"),
    ]
}

pub fn key_value_storage_columns() -> Vec<ColumnDef> {
    vec![
        id_column(),
        not_null_unique_col(ColumnType::Text, "key"),
        not_null_col(ColumnType::Text, "value"),
    ]
}

pub fn get_foreign_def_keyword_lookup() -> Vec<ForeignKeyDef> {
    vec![ForeignKeyDef {
        column: "text_id".to_string(),
        referenced_table: "texts".to_string(),
        referenced_column: "id".to_string(),
    }]
}

fn nullable_text_or_int(name: &str, column_type: &str) -> ColumnDef {
    ColumnDef {
        name: name.to_string(),
        column_type: column_type.to_string(),
        primary_key: false,
        not_null: false,
        unique: false,
        default_value: "".to_string(),
        autoincrement: false,
    }
}
