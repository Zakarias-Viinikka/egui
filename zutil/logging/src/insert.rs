use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{ColumnValue, InsertDataIn};
use protocol::row_col::Col;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::category_level::level_for;
use crate::session;

pub fn log(
    db: &LiveForever,
    category: &str,
    source: &str,
    message: &str,
) -> Result<(), DbError> {
    let session_id = session::get().to_string();
    let level = level_for(category);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    db.insert_data(InsertDataIn {
        table_name: "logs".to_string(),
        values: vec![
            ColumnValue {
                column_name: "timestamp".to_string(),
                value: Col::Integer(timestamp),
            },
            ColumnValue {
                column_name: "level".to_string(),
                value: Col::Text(level.to_string()),
            },
            ColumnValue {
                column_name: "category".to_string(),
                value: Col::Text(category.to_string()),
            },
            ColumnValue {
                column_name: "source".to_string(),
                value: Col::Text(source.to_string()),
            },
            ColumnValue {
                column_name: "session_id".to_string(),
                value: Col::Text(session_id),
            },
            ColumnValue {
                column_name: "message".to_string(),
                value: Col::Text(message.to_string()),
            },
            ColumnValue {
                column_name: "details".to_string(),
                value: Col::Null,
            },
            ColumnValue {
                column_name: "details_type".to_string(),
                value: Col::Null,
            },
        ],
    })
}
