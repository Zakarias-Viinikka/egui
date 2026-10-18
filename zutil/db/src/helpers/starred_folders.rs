use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{
    ColumnValue, DeleteRowIn, GetDataIn, InsertDataIn, SelectArgument, SelectArguments,
};
use protocol::row_col::Col;
use std::path::PathBuf;

pub fn read_all(db: &LiveForever) -> Result<Vec<PathBuf>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "starred_folders".to_string(),
        arguments: SelectArguments::Single(SelectArgument::All),
        columns_to_read: vec!["path".to_string()],
    })?;

    let mut paths = Vec::new();
    for row in out.rows {
        if let Some(col) = row.cols.first() {
            if let Ok(s) = col.as_str() {
                paths.push(PathBuf::from(s));
            }
        }
    }
    Ok(paths)
}

pub fn star(db: &LiveForever, path: &PathBuf) -> Result<(), DbError> {
    db.insert_data(InsertDataIn {
        table_name: "starred_folders".to_string(),
        values: vec![ColumnValue {
            column_name: "path".to_string(),
            value: Col::Text(path.to_string_lossy().to_string()),
        }],
    })?;
    Ok(())
}

pub fn unstar(db: &LiveForever, path: &PathBuf) -> Result<(), DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "starred_folders".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "path".to_string(),
            y: path.to_string_lossy().to_string(),
        }),
        columns_to_read: vec!["id".to_string()],
    })?;

    for row in out.rows {
        if let Some(id) = row.cols.first().and_then(|c| c.as_int().ok().copied()) {
            db.delete_row(DeleteRowIn {
                table_name: "starred_folders".to_string(),
                row_id: id.to_string(),
            })?;
        }
    }
    Ok(())
}
