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

#[test]
fn version4_adds_popularity_column_to_each_bumped_table() {
    use zutil_db::migration::schemas::{version3, version4};

    let v3_tables = version3::entire_table();
    let v4_tables = version4::entire_table();

    for name in ["texts", "templates", "projects", "categories"] {
        let v3 = v3_tables.iter().find(|t| t.table_name == name).unwrap();
        let v4 = v4_tables.iter().find(|t| t.table_name == name).unwrap();
        assert_eq!(v4.columns.len(), v3.columns.len() + 1, "{}", name);
        let last = v4.columns.last().unwrap();
        assert_eq!(last.name, "popularity_ctr");
        assert_eq!(last.column_type, "INTEGER");
        assert!(last.not_null);
        assert_eq!(last.default_value, "0");
    }
}

#[test]
fn version4_main_nav_clicks_shape_is_unchanged() {
    use zutil_db::migration::schemas::version4;

    let cols = version4::main_nav_clicks_columns();
    assert_eq!(cols.len(), 3);
    assert_eq!(cols[0].name, "id");
    assert_eq!(cols[1].name, "name");
    assert!(cols[1].unique);
    assert_eq!(cols[2].name, "popularity_ctr");
    assert_eq!(cols[2].column_type, "INTEGER");
    assert!(cols[2].not_null);
}

#[test]
fn version5_shortcuts_shape_is_unchanged() {
    use zutil_db::migration::schemas::version5;

    let cols = version5::shortcuts_columns();
    assert_eq!(cols.len(), 4);
    assert_eq!(cols[0].name, "id");
    assert_eq!(cols[1].name, "owner_kind");
    assert_eq!(cols[2].name, "owner_id");
    assert_eq!(cols[3].name, "combo");
}

#[test]
fn version6_error_log_shape_is_unchanged() {
    use zutil_db::migration::schemas::version6;

    let cols = version6::error_log_columns();
    let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["id", "timestamp", "screen", "location", "detail"]);
}
