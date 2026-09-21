use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{
    ColumnValue, GetDataIn, InsertDataIn, SelectArgument, SelectArguments,
};
use protocol::row_col::Col;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn insert_error(
    db: &LiveForever,
    screen: &str,
    location: &str,
    detail: &str,
) -> Result<(), DbError> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    db.insert_data(InsertDataIn {
        table_name: "error_log".to_string(),
        values: vec![
            ColumnValue { column_name: "timestamp".to_string(), value: Col::Integer(ts) },
            ColumnValue { column_name: "screen".to_string(), value: Col::Text(screen.to_string()) },
            ColumnValue { column_name: "location".to_string(), value: Col::Text(location.to_string()) },
            ColumnValue { column_name: "detail".to_string(), value: Col::Text(detail.to_string()) },
        ],
    })?;
    Ok(())
}

pub struct ErrorRow {
    pub timestamp: i64,
    pub screen: String,
    pub location: String,
    pub detail: String,
}

pub fn read_recent_errors(db: &LiveForever, limit: usize) -> Result<Vec<ErrorRow>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "error_log".to_string(),
        arguments: SelectArguments::Single(SelectArgument::All),
        columns_to_read: vec![
            "timestamp".to_string(),
            "screen".to_string(),
            "location".to_string(),
            "detail".to_string(),
        ],
    })?;
    let mut rows: Vec<ErrorRow> = out
        .rows
        .into_iter()
        .map(|r| ErrorRow {
            timestamp: r.cols.first().and_then(|c| c.as_int().ok().copied()).unwrap_or(0),
            screen: r.cols.get(1).and_then(|c| c.as_str().ok()).unwrap_or("").to_string(),
            location: r.cols.get(2).and_then(|c| c.as_str().ok()).unwrap_or("").to_string(),
            detail: r.cols.get(3).and_then(|c| c.as_str().ok()).unwrap_or("").to_string(),
        })
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.timestamp));
    rows.truncate(limit);
    Ok(rows)
}
