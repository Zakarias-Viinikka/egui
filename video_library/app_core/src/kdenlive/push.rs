use crate::error::{AppError, AppResult};
use crate::kdenlive::talker::{build_proxy, connect};
use std::path::Path;

pub fn push_to_kdenlive(
    source_file: &Path,
    original_id: i32,
    original_track: i32,
    original_position: i32,
) -> AppResult<i32> {
    let (conn, name) = connect()?;
    let proxy = build_proxy(&conn, &name)?;

    let path_str = source_file
        .to_str()
        .ok_or_else(|| AppError::new("source path not utf-8", "kdenlive::push", file!()))?
        .to_string();

    let imported = proxy
        .script_import_media(vec![path_str], "-1".to_string())
        .map_err(|e| AppError::new(format!("import: {e}"), "kdenlive::push", file!()))?;

    let bin_id = imported
        .first()
        .ok_or_else(|| AppError::new("import returned empty", "kdenlive::push", file!()))?
        .clone();

    let deleted = proxy
        .script_delete_timeline_clip(original_id)
        .map_err(|e| AppError::new(format!("delete: {e}"), "kdenlive::push", file!()))?;

    if !deleted {
        return Err(AppError::new(
            format!("delete of clip {original_id} returned false — original may have been modified"),
            "kdenlive::push",
            file!(),
        ));
    }

    let new_id = proxy
        .script_insert_clip(bin_id, original_track, original_position)
        .map_err(|e| AppError::new(format!("insert: {e}"), "kdenlive::push", file!()))?;

    if new_id < 0 {
        return Err(AppError::new(format!("insert returned {new_id}"), "kdenlive::push", file!()));
    }

    Ok(new_id)
}
