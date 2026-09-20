use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{ColumnValue, GetDataIn, InsertDataIn, SelectArgument, SelectArguments};
use protocol::row_col::{Col, Row};

pub fn normalize_path(raw: &str) -> String {
    let trimmed = raw.trim();
    let unquoted = strip_matching_quotes(trimmed);
    expand_tilde(unquoted.trim())
}

fn strip_matching_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

fn expand_tilde(s: &str) -> String {
    if s == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| s.to_string());
    }
    if let Some(rest) = s.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        return format!("{}/{}", home, rest);
    }
    s.to_string()
}

pub fn new_row_project(
    title: String,
    path: String,
    launch_zed: bool,
    launch_adstud: bool,
) -> InsertDataIn {
    let clean_path = normalize_path(&path);
    InsertDataIn {
        table_name: "projects".to_string(),
        values: vec![
            ColumnValue {
                column_name: "title".to_string(),
                value: Col::Text(title),
            },
            ColumnValue {
                column_name: "path".to_string(),
                value: Col::Text(clean_path),
            },
            ColumnValue {
                column_name: "launch_zed".to_string(),
                value: Col::Integer(if launch_zed { 1 } else { 0 }),
            },
            ColumnValue {
                column_name: "launch_adstud".to_string(),
                value: Col::Integer(if launch_adstud { 1 } else { 0 }),
            },
        ],
    }
}

pub fn read_all_projects(db: &LiveForever) -> Result<Vec<Row>, DbError> {
    Ok(db
        .get_data(GetDataIn {
            table_name: "projects".to_string(),
            arguments: SelectArguments::Single(SelectArgument::All),
            columns_to_read: Vec::new(),
        })?
        .rows)
}
