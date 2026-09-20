#[macro_use]
mod macros;

mod common;
use common::*;

use zutil_db::migration::schemas::version0::get_foreign_def_keyword_lookup;

#[test]
fn keyword_lookup_table_is_correct() {
    let db = setup_fresh_db("test_keyword_lookup.sqlite");
    create_keyword_lookup_table!(&db);

    let result = check_table(CheckTable {
        table_name: "keyword_lookup",
        db: &db,
    });
    let expected_result = vec![
        col!(0, "id", "INTEGER", pk),
        col!(1, "text_id", "INTEGER", not_null),
        col!(2, "keyword", "TEXT", not_null),
    ];
    assert_eq!(result, expected_result);

    let result = missing_fk_columns(MissingFkColumns {
        table_name: "keyword_lookup",
        db: &db,
    });
    let expected_result: Vec<String> = vec![];
    assert_eq!(result, expected_result);
}

// ============================================================
// HELPERS
// ============================================================

struct MissingFkColumns<'a> {
    table_name: &'a str,
    db: &'a db_wrapper::mascot::LiveForever,
}

fn missing_fk_columns(params: MissingFkColumns) -> Vec<String> {
    let cols = check_table(CheckTable {
        table_name: params.table_name,
        db: params.db,
    });
    get_foreign_def_keyword_lookup()
        .iter()
        .filter(|fk| !cols.iter().any(|c| c.name == fk.column))
        .map(|fk| fk.column.clone())
        .collect()
}
