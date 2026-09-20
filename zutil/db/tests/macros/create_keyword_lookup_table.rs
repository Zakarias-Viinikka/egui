macro_rules! create_keyword_lookup_table {
    ($db:expr) => {
        $db.create_table(protocol::payload::CreateTableIn {
            table_name: "keyword_lookup".to_string(),
            columns: zutil_db::migration::schemas::version0::keyword_lookup_columns(),
        })
        .unwrap();
    };
}
