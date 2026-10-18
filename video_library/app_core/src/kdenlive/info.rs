use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::path::PathBuf;
use zbus::zvariant::OwnedValue;

pub struct TimelineClipInfo {
    pub id: i32,
    pub bin_id: String,
    pub url: PathBuf,
    pub track_id: i32,
    pub position: i32,
    pub in_frame: i32,
    pub out_frame: i32,
    pub duration: i32,
    pub name: String,
}

impl TimelineClipInfo {
    pub fn from_dbus(map: &HashMap<String, OwnedValue>) -> AppResult<Self> {
        Ok(Self {
            id: get_i32(map, "id")?,
            bin_id: get_string(map, "binId")?,
            url: PathBuf::from(get_string(map, "url")?),
            track_id: get_i32(map, "trackId")?,
            position: get_i32(map, "position")?,
            in_frame: get_i32(map, "in")?,
            out_frame: get_i32(map, "out")?,
            duration: get_i32(map, "duration")?,
            name: get_string(map, "name")?,
        })
    }
}

fn get_i32(map: &HashMap<String, OwnedValue>, key: &str) -> AppResult<i32> {
    map.get(key)
        .and_then(|v| v.downcast_ref::<i32>().ok())
        .ok_or_else(|| AppError::new(format!("missing or invalid '{key}'"), "kdenlive::info::from_dbus", file!()))
}

fn get_string(map: &HashMap<String, OwnedValue>, key: &str) -> AppResult<String> {
    map.get(key)
        .and_then(|v| v.downcast_ref::<String>().ok())
        .ok_or_else(|| AppError::new(format!("missing or invalid '{key}'"), "kdenlive::info::from_dbus", file!()))
}
