#[macro_use]
mod macros;

mod common;
use common::*;

#[test]
fn texts_table_has_expected_shape() {
    let db = setup_fresh_db("test_texts.sqlite");
    create_texts_table!(&db);

    let result = check_table(CheckTable {
        table_name: "texts",
        db: &db,
    });
    let expected_result = vec![
        col!(0, "id", "INTEGER", pk),
        col!(1, "title", "TEXT", not_null),
        col!(2, "body", "TEXT", not_null),
        col!(3, "category_id", "INTEGER"),
        col!(4, "meta_category_id", "INTEGER"),
        col!(5, "type_of_text", "TEXT"),
    ];
    assert_eq!(result, expected_result);
}
