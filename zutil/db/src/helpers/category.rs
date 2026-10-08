use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::payload::{
    ColumnValue, GetDataIn, InsertDataIn, SelectArgument, SelectArguments,
};
use protocol::row_col::Col;

pub const TYPE_NORMAL: &str = "normal";
pub const TYPE_META: &str = "meta";

/// Case-insensitive lookup by name within a type.
/// SQLite `=` is case-sensitive for TEXT, so we fetch all rows of the type
/// and match in Rust with `eq_ignore_ascii_case`.
pub fn find_category_id_by_name_and_type(
    db: &LiveForever,
    name: &str,
    type_of_category: &str,
) -> Result<Option<i64>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "categories".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "type_of_category".to_string(),
            y: type_of_category.to_string(),
        }),
        columns_to_read: vec!["id".to_string(), "name".to_string()],
    })?;

    let needle = name.trim();
    for row in out.rows {
        let id = match row.cols.first().and_then(|c| c.as_int().ok().copied()) {
            Some(v) => v,
            None => continue,
        };
        let row_name = match row.cols.get(1).and_then(|c| c.as_str().ok()) {
            Some(s) => s,
            None => continue,
        };
        if row_name.eq_ignore_ascii_case(needle) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// If another category exists with the same case-insensitive name and same
/// type, return its id. `exclude_id` lets a rename skip its own row.
pub fn category_name_taken(
    db: &LiveForever,
    name: &str,
    type_of_category: &str,
    exclude_id: Option<i64>,
) -> Result<Option<i64>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "categories".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "type_of_category".to_string(),
            y: type_of_category.to_string(),
        }),
        columns_to_read: vec!["id".to_string(), "name".to_string()],
    })?;
    let needle = name.trim();
    for row in out.rows {
        let id = match row.cols.first().and_then(|c| c.as_int().ok().copied()) {
            Some(v) => v,
            None => continue,
        };
        if Some(id) == exclude_id {
            continue;
        }
        let row_name = match row.cols.get(1).and_then(|c| c.as_str().ok()) {
            Some(s) => s,
            None => continue,
        };
        if row_name.eq_ignore_ascii_case(needle) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Returns groups of categories that share the same lowercase name and the
/// same type. Never merges — just reports.
pub fn find_category_collisions(
    db: &LiveForever,
) -> Result<Vec<(String, String, Vec<i64>)>, DbError> {
    let triples = read_all_categories_with_ids(db)?;
    let mut map: std::collections::HashMap<(String, String), Vec<i64>> =
        std::collections::HashMap::new();
    for (id, name, ty) in &triples {
        let key = (name.to_lowercase(), ty.clone());
        map.entry(key).or_default().push(*id);
    }
    let mut out = Vec::new();
    for ((_lower, ty), ids) in map {
        if ids.len() > 1 {
            let display = triples
                .iter()
                .find(|(id, _, _)| ids.contains(id))
                .map(|(_, n, _)| n.clone())
                .unwrap_or_default();
            out.push((display, ty, ids));
        }
    }
    Ok(out)
}

pub fn get_or_create_category_id(
    db: &LiveForever,
    name: &str,
    type_of_category: &str,
) -> Result<i64, DbError> {
    let trimmed = name.trim();
    if let Some(id) = find_category_id_by_name_and_type(db, trimmed, type_of_category)? {
        let existing = read_category_name_by_id(db, id)?.unwrap_or_default();
        if existing == trimmed {
            return Ok(id);
        }
        return Err(DbError::IllegalInput(format!(
            "category {:?} already exists as {:?} (id {}). Match the existing case.",
            trimmed, existing, id
        )));
    }
    db.insert_data(InsertDataIn {
        table_name: "categories".to_string(),
        values: vec![
            ColumnValue {
                column_name: "name".to_string(),
                value: Col::Text(trimmed.to_string()),
            },
            ColumnValue {
                column_name: "type_of_category".to_string(),
                value: Col::Text(type_of_category.to_string()),
            },
        ],
    })?;
    find_category_id_by_name_and_type(db, trimmed, type_of_category)?
        .ok_or_else(|| DbError::BadCode("category vanished after insert".to_string()))
}

pub fn read_categories_with_ids_by_type(
    db: &LiveForever,
    type_of_category: &str,
) -> Result<Vec<(i64, String)>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "categories".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "type_of_category".to_string(),
            y: type_of_category.to_string(),
        }),
        columns_to_read: vec!["id".to_string(), "name".to_string()],
    })?;

    let mut pairs = Vec::new();
    for row in out.rows {
        let id = match row.cols.first().and_then(|c| c.as_int().ok()) {
            Some(v) => *v,
            None => continue,
        };
        let name = match row.cols.get(1).and_then(|c| c.as_str().ok()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        pairs.push((id, name));
    }
    Ok(pairs)
}

pub fn read_all_categories_with_ids(
    db: &LiveForever,
) -> Result<Vec<(i64, String, String)>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "categories".to_string(),
        arguments: SelectArguments::Single(SelectArgument::All),
        columns_to_read: vec![
            "id".to_string(),
            "name".to_string(),
            "type_of_category".to_string(),
        ],
    })?;

    let mut triples = Vec::new();
    for row in out.rows {
        let id = match row.cols.first().and_then(|c| c.as_int().ok()) {
            Some(v) => *v,
            None => continue,
        };
        let name = match row.cols.get(1).and_then(|c| c.as_str().ok()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let t = match row.cols.get(2).and_then(|c| c.as_str().ok()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        triples.push((id, name, t));
    }
    Ok(triples)
}

pub fn read_category_type_by_id(
    db: &LiveForever,
    id: i64,
) -> Result<Option<String>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "categories".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "id".to_string(),
            y: id.to_string(),
        }),
        columns_to_read: vec!["type_of_category".to_string()],
    })?;
    let row = match out.rows.into_iter().next() {
        Some(r) => r,
        None => return Ok(None),
    };
    let col = match row.cols.into_iter().next() {
        Some(c) => c,
        None => return Ok(None),
    };
    Ok(col.as_str().ok().map(|s| s.to_string()))
}

pub fn read_category_name_by_id(
    db: &LiveForever,
    id: i64,
) -> Result<Option<String>, DbError> {
    let out = db.get_data(GetDataIn {
        table_name: "categories".to_string(),
        arguments: SelectArguments::Single(SelectArgument::XEqualY {
            x: "id".to_string(),
            y: id.to_string(),
        }),
        columns_to_read: vec!["name".to_string()],
    })?;

    let row = match out.rows.into_iter().next() {
        Some(r) => r,
        None => return Ok(None),
    };
    let col = match row.cols.into_iter().next() {
        Some(c) => c,
        None => return Ok(None),
    };
    Ok(col.as_str().ok().map(|s| s.to_string()))
}


/// Empty name -> None (no category). Non-empty -> id, creating the category if needed.
pub fn category_id_opt(
    db: &LiveForever,
    name: &str,
    type_of_category: &str,
) -> Result<Option<i64>, DbError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(get_or_create_category_id(db, trimmed, type_of_category)?))
}

/// Empty name -> Col::Null (clears the column). Non-empty -> Col::Integer(id).
pub fn category_col(
    db: &LiveForever,
    name: &str,
    type_of_category: &str,
) -> Result<Col, DbError> {
    match category_id_opt(db, name, type_of_category)? {
        Some(id) => Ok(Col::Integer(id)),
        None => Ok(Col::Null(protocol::row_col::StructRepresentingNull {})),
    }
}
