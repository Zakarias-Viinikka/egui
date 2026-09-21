use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{
    ColumnValue, DeleteRowIn, GetDataIn, InsertDataIn, SelectArgument, SelectArguments,
};
use protocol::row_col::Col;

pub const OWNER_MAIN_NAV: &str = "main_nav";
pub const OWNER_CATEGORY: &str = "category";
pub const OWNER_TEXT: &str = "text";
pub const OWNER_TEMPLATE: &str = "template";
pub const OWNER_PROJECT: &str = "project";

/// Returns the combo bound to (owner_kind, owner_id), if any.
pub fn read_shortcut(
    db: &LiveForever,
    owner_kind: &str,
    owner_id: &str,
) -> Result<Option<String>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "shortcuts".to_string(),
        arguments: SelectArguments::Two {
            first: SelectArgument::XEqualY {
                x: "owner_kind".to_string(),
                y: owner_kind.to_string(),
            },
            join: protocol::payload::JoinType::And,
            second: SelectArgument::XEqualY {
                x: "owner_id".to_string(),
                y: owner_id.to_string(),
            },
        },
        columns_to_read: vec!["combo".to_string()],
    })?;
    Ok(out
        .rows
        .first()
        .and_then(|r| r.cols.first())
        .and_then(|c| c.as_str().ok())
        .map(|s| s.to_string()))
}

/// Returns the owner (kind, id) that already uses this combo, if any.
pub fn combo_owner(
    db: &LiveForever,
    combo: &str,
) -> Result<Option<(String, String)>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "shortcuts".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "combo".to_string(),
            y: combo.to_string(),
        }),
        columns_to_read: vec!["owner_kind".to_string(), "owner_id".to_string()],
    })?;
    Ok(out.rows.first().and_then(|r| {
        let kind = r.cols.first().and_then(|c| c.as_str().ok())?.to_string();
        let id = r.cols.get(1).and_then(|c| c.as_str().ok())?.to_string();
        Some((kind, id))
    }))
}

/// Sets the combo for (owner_kind, owner_id). Replaces whatever was there.
/// Collision checking is done by the caller, since only it knows which
/// circle is on screen.
pub fn set_shortcut(
    db: &LiveForever,
    owner_kind: &str,
    owner_id: &str,
    combo: &str,
) -> Result<(), DbError> {
    clear_shortcut(db, owner_kind, owner_id)?;
    db.insert_data(InsertDataIn {
        table_name: "shortcuts".to_string(),
        values: vec![
            ColumnValue {
                column_name: "owner_kind".to_string(),
                value: Col::Text(owner_kind.to_string()),
            },
            ColumnValue {
                column_name: "owner_id".to_string(),
                value: Col::Text(owner_id.to_string()),
            },
            ColumnValue {
                column_name: "combo".to_string(),
                value: Col::Text(combo.to_string()),
            },
        ],
    })?;
    Ok(())
}

pub fn clear_shortcut(
    db: &LiveForever,
    owner_kind: &str,
    owner_id: &str,
) -> Result<(), DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "shortcuts".to_string(),
        arguments: SelectArguments::Two {
            first: SelectArgument::XEqualY {
                x: "owner_kind".to_string(),
                y: owner_kind.to_string(),
            },
            join: protocol::payload::JoinType::And,
            second: SelectArgument::XEqualY {
                x: "owner_id".to_string(),
                y: owner_id.to_string(),
            },
        },
        columns_to_read: vec!["id".to_string()],
    })?;
    for row in out.rows {
        if let Some(id) = row.cols.first().and_then(|c| c.as_int().ok().copied()) {
            db.delete_row(DeleteRowIn {
                table_name: "shortcuts".to_string(),
                row_id: id.to_string(),
            })?;
        }
    }
    Ok(())
}
