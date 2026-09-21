mod common;
use common::*;

use zutil_db::init::setup_fts5;
use zutil_db::migration::helpers::{
    assert_migrated_db_matches_fresh_db, create_table_from_scratch,
};
use zutil_db::migration::schema_versions::{CURRENT_VERSION, SchemaVersion};
use zutil_db::migration::schemas::{current, version0, version3};
use zutil_db::migration::update_from_old_schema::update_until_newest_version;

#[test]
fn migration_chain_reaches_current_version() {
    let db = setup_fresh_db("test_migration.sqlite");
    create_table_from_scratch(version0::entire_table(), &db).unwrap();

    let result = update_until_newest_version(SchemaVersion::Version0, &db);
    let expected_result = CURRENT_VERSION;
    assert_eq!(result, expected_result);

    assert_migrated_db_matches_fresh_db(current::entire_table(), &db);
}

#[test]
fn migrated_db_gets_fts5() {
    let db = setup_fresh_db("test_migration_fts5.sqlite");
    create_table_from_scratch(version0::entire_table(), &db).unwrap();
    update_until_newest_version(SchemaVersion::Version0, &db);
    setup_fts5(&db);

    let result = has_fts5_table(&db);
    let expected_result = true;
    assert_eq!(result, expected_result);
}

#[test]
fn fresh_db_and_migrated_db_have_same_fts5() {
    let fresh = setup_fresh_db("test_migration_fresh.sqlite");
    create_table_from_scratch(current::entire_table(), &fresh).unwrap();
    setup_fts5(&fresh);

    let migrated = setup_fresh_db("test_migration_migrated.sqlite");
    create_table_from_scratch(version0::entire_table(), &migrated).unwrap();
    update_until_newest_version(SchemaVersion::Version0, &migrated);
    setup_fts5(&migrated);

    let result = (has_fts5_table(&fresh), has_fts5_table(&migrated));
    let expected_result = (true, true);
    assert_eq!(result, expected_result);
}

#[test]
fn migration_v3_to_v4_adds_popularity_and_main_nav_clicks() {
    let db = setup_fresh_db("test_migration_v3_to_v4.sqlite");
    create_table_from_scratch(version3::entire_table(), &db).unwrap();
    update_until_newest_version(SchemaVersion::Version3, &db);

    for table in ["texts", "templates", "projects", "categories"] {
        let cols = check_table(CheckTable { table_name: table, db: &db });
        assert!(
            cols.iter().any(|c| c.name == "popularity_ctr"),
            "{} missing popularity_ctr",
            table
        );
    }

    let names = db.list_tables().unwrap().table_names;
    assert!(
        names.contains(&"main_nav_clicks".to_string()),
        "main_nav_clicks missing"
    );

    assert_migrated_db_matches_fresh_db(current::entire_table(), &db);
}

// HELPERS

fn has_fts5_table(db: &db_wrapper::mascot::LiveForever) -> bool {
    db.list_tables()
        .unwrap()
        .table_names
        .iter()
        .any(|t| t.starts_with("fts5_"))
}
