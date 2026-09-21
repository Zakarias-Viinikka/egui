use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{
    ColumnValue, EditColInRowIn, GetDataIn, InsertDataIn, SelectArgument, SelectArguments,
};
use protocol::row_col::Col;

pub fn increment_text(db: &LiveForever, id: i64) -> Result<(), DbError> {
    bump_existing(db, "texts", id)
}

pub fn increment_template(db: &LiveForever, id: i64) -> Result<(), DbError> {
    bump_existing(db, "templates", id)
}

pub fn increment_project(db: &LiveForever, id: i64) -> Result<(), DbError> {
    bump_existing(db, "projects", id)
}

pub fn increment_category(db: &LiveForever, id: i64) -> Result<(), DbError> {
    bump_existing(db, "categories", id)
}

pub fn read_main_nav_counter(db: &LiveForever, name: &str) -> Result<i64, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "main_nav_clicks".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "name".to_string(),
            y: name.to_string(),
        }),
        columns_to_read: vec!["popularity_ctr".to_string()],
    })?;
    Ok(out
        .rows
        .first()
        .and_then(|r| r.cols.first())
        .and_then(|c| c.as_int().ok().copied())
        .unwrap_or(0))
}

pub fn increment_main_nav(db: &LiveForever, name: &str) -> Result<(), DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "main_nav_clicks".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "name".to_string(),
            y: name.to_string(),
        }),
        columns_to_read: vec!["id".to_string(), "popularity_ctr".to_string()],
    })?;

    let existing = out.rows.into_iter().next();
    match existing {
        None => {
            db.insert_data(InsertDataIn {
                table_name: "main_nav_clicks".to_string(),
                values: vec![
                    ColumnValue {
                        column_name: "name".to_string(),
                        value: Col::Text(name.to_string()),
                    },
                    ColumnValue {
                        column_name: "popularity_ctr".to_string(),
                        value: Col::Integer(1),
                    },
                ],
            })?;
        }
        Some(row) => {
            let id = row.cols.first().and_then(|c| c.as_int().ok().copied()).unwrap_or(0);
            let ctr = row.cols.get(1).and_then(|c| c.as_int().ok().copied()).unwrap_or(0);
            db.edit_col_in_row(EditColInRowIn {
                table_name: "main_nav_clicks".to_string(),
                row_id: id.to_string(),
                column: "popularity_ctr".to_string(),
                new_value: Col::Integer(ctr + 1),
            })?;
        }
    }
    Ok(())
}

fn bump_existing(db: &LiveForever, table: &str, id: i64) -> Result<(), DbError> {
    let out = db.get_data(GetDataIn {
        table_name: table.to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "id".to_string(),
            y: id.to_string(),
        }),
        columns_to_read: vec!["popularity_ctr".to_string()],
    })?;
    let current = out
        .rows
        .first()
        .and_then(|r| r.cols.first())
        .and_then(|c| c.as_int().ok().copied())
        .unwrap_or(0);
    db.edit_col_in_row(EditColInRowIn {
        table_name: table.to_string(),
        row_id: id.to_string(),
        column: "popularity_ctr".to_string(),
        new_value: Col::Integer(current + 1),
    })?;
    Ok(())
}

pub fn read_counter(db: &LiveForever, table: &str, id: i64) -> Result<u32, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: table.to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "id".to_string(),
            y: id.to_string(),
        }),
        columns_to_read: vec!["popularity_ctr".to_string()],
    })?;
    Ok(out
        .rows
        .first()
        .and_then(|r| r.cols.first())
        .and_then(|c| c.as_int().ok().copied())
        .map(|v| v as u32)
        .unwrap_or(0))
}
