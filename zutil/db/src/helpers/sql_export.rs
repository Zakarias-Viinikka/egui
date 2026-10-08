use db_wrapper::mascot::LiveForever;
use protocol::error::DbError;
use protocol::new_table::ColumnDef;
use protocol::payload::{GetDataIn, SelectArgument, SelectArguments};
use protocol::row_col::{Col, Row};

use crate::tables::{
    tbl_categories, tbl_keyword_lookup, tbl_main_nav_clicks, tbl_template_fills, tbl_templates,
    tbl_texts,
};

fn quote_str(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn col_to_sql(col: &Col) -> String {
    match col {
        Col::Null(_) => "NULL".to_string(),
        Col::Integer(i) => i.to_string(),
        Col::Real(r) => r.to_string(),
        Col::Text(s) => quote_str(s),
        Col::Blob(b) => format!("X'{}'", hex(b)),
    }
}

fn read_all_rows(db: &LiveForever, table: &str) -> Result<Vec<Row>, DbError> {
    Ok(db
        .get_data(GetDataIn {
            table_name: table.to_string(),
            arguments: SelectArguments::Single(SelectArgument::All),
            columns_to_read: Vec::new(),
        })?
        .rows)
}

fn read_rows_where_eq(
    db: &LiveForever,
    table: &str,
    col: &str,
    val: i64,
) -> Result<Vec<Row>, DbError> {
    Ok(db
        .get_data(GetDataIn {
            table_name: table.to_string(),
            arguments: SelectArguments::Single(SelectArgument::XEqualY {
                x: col.to_string(),
                y: val.to_string(),
            }),
            columns_to_read: Vec::new(),
        })?
        .rows)
}

fn columns_of(cols: &[ColumnDef]) -> Vec<String> {
    cols.iter().map(|c| c.name.clone()).collect()
}

fn emit_table(
    out: &mut String,
    table: &str,
    col_defs: &[ColumnDef],
    rows: &[Row],
) {
    if rows.is_empty() {
        return;
    }
    let cols = columns_of(col_defs);
    out.push_str(&format!("-- {}\n", table));
    for row in rows {
        let vals: Vec<String> = row.cols.iter().map(col_to_sql).collect();
        out.push_str(&format!(
            "INSERT INTO {} ({}) VALUES ({});\n",
            table,
            cols.join(", "),
            vals.join(", ")
        ));
    }
    out.push('\n');
}

/// All rows of the tables that carry user data, in FK-safe order.
pub fn full_export(db: &LiveForever) -> Result<String, DbError> {
    let mut out = String::new();
    out.push_str("BEGIN TRANSACTION;\n\n");

    let cats = read_all_rows(db, tbl_categories::TABLE_NAME)?;
    emit_table(
        &mut out,
        tbl_categories::TABLE_NAME,
        &tbl_categories::new_table_categories(),
        &cats,
    );

    let texts = read_all_rows(db, tbl_texts::TABLE_NAME)?;
    emit_table(
        &mut out,
        tbl_texts::TABLE_NAME,
        &tbl_texts::new_table_texts(),
        &texts,
    );

    let templates = read_all_rows(db, tbl_templates::TABLE_NAME)?;
    emit_table(
        &mut out,
        tbl_templates::TABLE_NAME,
        &tbl_templates::new_table_templates(),
        &templates,
    );

    let kws = read_all_rows(db, tbl_keyword_lookup::TABLE_NAME)?;
    emit_table(
        &mut out,
        tbl_keyword_lookup::TABLE_NAME,
        &tbl_keyword_lookup::new_table_keyword_lookup(),
        &kws,
    );

    let fills = read_all_rows(db, tbl_template_fills::TABLE_NAME)?;
    emit_table(
        &mut out,
        tbl_template_fills::TABLE_NAME,
        &tbl_template_fills::new_table_template_fills(),
        &fills,
    );

    let nav = read_all_rows(db, tbl_main_nav_clicks::TABLE_NAME)?;
    emit_table(
        &mut out,
        tbl_main_nav_clicks::TABLE_NAME,
        &tbl_main_nav_clicks::new_table_main_nav_clicks(),
        &nav,
    );

    out.push_str("COMMIT;\n");
    Ok(out)
}

/// One text, its category rows, and its keyword_lookup rows.
pub fn scoped_text_export(db: &LiveForever, text_id: i64) -> Result<String, DbError> {
    let mut out = String::new();
    out.push_str("BEGIN TRANSACTION;\n\n");

    let text_rows = read_rows_where_eq(db, tbl_texts::TABLE_NAME, "id", text_id)?;
    emit_table(
        &mut out,
        tbl_texts::TABLE_NAME,
        &tbl_texts::new_table_texts(),
        &text_rows,
    );

    // category_id and meta_category_id are columns 3 and 4 in texts schema order
    let mut cat_ids: Vec<i64> = Vec::new();
    if let Some(r) = text_rows.first() {
        for idx in [3usize, 4] {
            if let Some(id) = r.cols.get(idx).and_then(|c| c.as_int().ok().copied()) {
                if !cat_ids.contains(&id) {
                    cat_ids.push(id);
                }
            }
        }
    }
    for cid in &cat_ids {
        let rows = read_rows_where_eq(db, tbl_categories::TABLE_NAME, "id", *cid)?;
        emit_table(
            &mut out,
            tbl_categories::TABLE_NAME,
            &tbl_categories::new_table_categories(),
            &rows,
        );
    }

    let kw_rows = read_rows_where_eq(db, tbl_keyword_lookup::TABLE_NAME, "text_id", text_id)?;
    emit_table(
        &mut out,
        tbl_keyword_lookup::TABLE_NAME,
        &tbl_keyword_lookup::new_table_keyword_lookup(),
        &kw_rows,
    );

    out.push_str("COMMIT;\n");
    Ok(out)
}
