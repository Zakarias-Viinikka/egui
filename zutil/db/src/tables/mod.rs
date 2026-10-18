//! One file per table. To add a new table:
//!
//! 1. Create `tbl_<name>.rs` with:
//!    - `pub const TABLE_NAME: &str = "<name>";`
//!    - `pub fn new_table_<name>() -> Vec<ColumnDef>` (use `id_column()`,
//!      `not_null_col`, `not_null_unique_col`, or the local helpers in the
//!      file's header for nullable/int/text columns)
//!    - `pub fn foreign_keys() -> Vec<ForeignKeyDef>` if the table has any
//!    - one `SchemaColumn<T>` const per column, with `name` matching the
//!      string in `new_table_<name>()`
//! 2. Add the file to the module list below.
//! 3. Add an `ADbTable { ... }` entry to `entire_table()` using the const
//!    `TABLE_NAME`, `new_table_<name>()`, and `foreign_keys()`.
//! 4. Bump the schema version: add a `versionN.rs` under
//!    `migration/schemas/` and register it in `schema_versions.rs` and
//!    `update_from_old_schema.rs`. Existing versions are frozen — never
//!    edit them.
//! 5. Reference column names by const, not by literal string. Helpers in
//!    `db/src/helpers/` do this — follow that pattern.
//!
//! `current.rs` points `CURRENT_VERSION` at `tables::entire_table()`, so
//! fresh databases get the shape described here, and the migration chain
//! rebuilds old databases into the same shape.

pub mod tbl_categories;
pub mod tbl_texts;
pub mod tbl_keyword_lookup;
pub mod tbl_logs;
pub mod tbl_key_value_storage;
pub mod tbl_templates;
pub mod tbl_template_fills;
pub mod tbl_projects;
pub mod tbl_main_nav_clicks;
pub mod tbl_shortcuts;
pub mod tbl_error_log;
pub mod tbl_starred_folders;

use crate::migration::helpers::ADbTable;

pub fn entire_table() -> Vec<ADbTable> {
    vec![
        ADbTable { table_name: tbl_categories::TABLE_NAME.to_string(), columns: tbl_categories::new_table_categories(), foreign_keys: Vec::new() },
        ADbTable { table_name: tbl_texts::TABLE_NAME.to_string(), columns: tbl_texts::new_table_texts(), foreign_keys: tbl_texts::foreign_keys() },
        ADbTable { table_name: tbl_keyword_lookup::TABLE_NAME.to_string(), columns: tbl_keyword_lookup::new_table_keyword_lookup(), foreign_keys: tbl_keyword_lookup::foreign_keys() },
        ADbTable { table_name: tbl_logs::TABLE_NAME.to_string(), columns: tbl_logs::new_table_logs(), foreign_keys: Vec::new() },
        ADbTable { table_name: tbl_key_value_storage::TABLE_NAME.to_string(), columns: tbl_key_value_storage::new_table_key_value_storage(), foreign_keys: Vec::new() },
        ADbTable { table_name: tbl_templates::TABLE_NAME.to_string(), columns: tbl_templates::new_table_templates(), foreign_keys: tbl_templates::foreign_keys() },
        ADbTable { table_name: tbl_template_fills::TABLE_NAME.to_string(), columns: tbl_template_fills::new_table_template_fills(), foreign_keys: tbl_template_fills::foreign_keys() },
        ADbTable { table_name: tbl_projects::TABLE_NAME.to_string(), columns: tbl_projects::new_table_projects(), foreign_keys: Vec::new() },
        ADbTable { table_name: tbl_main_nav_clicks::TABLE_NAME.to_string(), columns: tbl_main_nav_clicks::new_table_main_nav_clicks(), foreign_keys: Vec::new() },
        ADbTable { table_name: tbl_shortcuts::TABLE_NAME.to_string(), columns: tbl_shortcuts::new_table_shortcuts(), foreign_keys: Vec::new() },
        ADbTable { table_name: tbl_error_log::TABLE_NAME.to_string(), columns: tbl_error_log::new_table_error_log(), foreign_keys: Vec::new() },
        ADbTable { table_name: tbl_starred_folders::TABLE_NAME.to_string(), columns: tbl_starred_folders::new_table_starred_folders(), foreign_keys: Vec::new() },
    ]
}
