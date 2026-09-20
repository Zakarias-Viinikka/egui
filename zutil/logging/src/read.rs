use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{GetDataIn, SelectArgument, SelectArguments};
use protocol::row_col::Row;

/// Sorted by severity (highest first), ties broken by recency. Not chronological.
pub fn top_logs_for_session(
    db: &LiveForever,
    session_id: &str,
    limit: usize,
) -> Result<Vec<Row>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "logs".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "session_id".to_string(),
            y: session_id.to_string(),
        }),
        columns_to_read: Vec::new(),
    })?;

    let mut rows = out.rows;
    rows.sort_by(|a, b| {
        level_of(b)
            .cmp(&level_of(a))
            .then(timestamp_of(b).cmp(&timestamp_of(a)))
    });
    rows.truncate(limit);
    Ok(rows)
}

fn level_of(row: &Row) -> i64 {
    row.cols
        .get(2)
        .and_then(|c| c.as_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

fn timestamp_of(row: &Row) -> i64 {
    row.cols
        .get(1)
        .and_then(|c| c.as_int().ok())
        .copied()
        .unwrap_or(0)
}
