use protocol::new_table::ColumnDef;
use zutil_db::migration::schemas::version0;

#[test]
fn texts_schema_is_unchanged() {
    let result = version0::texts_columns();
    let expected_result = vec![
        col("id", "INTEGER", true, true, true),
        col("title", "TEXT", false, true, false),
        col("body", "TEXT", false, true, false),
        col("category_id", "INTEGER", false, false, false),
        col("meta_category_id", "INTEGER", false, false, false),
        col("type_of_text", "TEXT", false, false, false),
    ];
    assert_eq!(result, expected_result);
}

#[test]
fn keyword_lookup_schema_is_unchanged() {
    let result = version0::keyword_lookup_columns();
    let expected_result = vec![
        col("id", "INTEGER", true, true, true),
        col("text_id", "INTEGER", false, true, false),
        col("keyword", "TEXT", false, true, false),
    ];
    assert_eq!(result, expected_result);
}

fn col(
    name: &str,
    column_type: &str,
    primary_key: bool,
    not_null: bool,
    autoincrement: bool,
) -> ColumnDef {
    ColumnDef {
        name: name.to_string(),
        column_type: column_type.to_string(),
        primary_key,
        not_null,
        unique: false,
        default_value: String::new(),
        autoincrement,
    }
}
