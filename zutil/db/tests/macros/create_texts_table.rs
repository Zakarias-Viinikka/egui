macro_rules! create_texts_table {
    ($db:expr) => {
        $db.create_table(protocol::payload::CreateTableIn {
            table_name: "texts".to_string(),
            columns: zutil_db::migration::schemas::version0::texts_columns(),
        })
        .unwrap();
    };
}
