macro_rules! col {
    ($cid:expr, $name:expr, $type_name:expr) => {
        protocol::payload::TableColumnInfo {
            cid: $cid,
            name: $name.to_string(),
            type_name: $type_name.to_string(),
            not_null: false,
            default_value: None,
            primary_key: false,
        }
    };
    ($cid:expr, $name:expr, $type_name:expr, not_null) => {
        protocol::payload::TableColumnInfo {
            cid: $cid,
            name: $name.to_string(),
            type_name: $type_name.to_string(),
            not_null: true,
            default_value: None,
            primary_key: false,
        }
    };
    ($cid:expr, $name:expr, $type_name:expr, pk) => {
        protocol::payload::TableColumnInfo {
            cid: $cid,
            name: $name.to_string(),
            type_name: $type_name.to_string(),
            not_null: true,
            default_value: None,
            primary_key: true,
        }
    };
}
