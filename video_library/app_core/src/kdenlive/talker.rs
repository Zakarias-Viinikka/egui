use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

#[derive(Clone, Debug, PartialEq)]
pub enum KdenliveStatus {
    Checking,
    Connected { name: String },
    NotRunning,
    Multiple { count: usize, names: Vec<String> },
    Error(String),
}

#[zbus::proxy(
    interface = "org.kde.kdenlive.MainWindow",
    default_service = "org.kde.kdenlive",
    default_path = "/MainWindow"
)]
pub trait KdenliveMainWindow {
    #[zbus(name = "scriptGetSelection")]
    fn script_get_selection(&self) -> zbus::Result<Vec<OwnedValue>>;
    #[zbus(name = "scriptGetTimelineClipInfo")]
    fn script_get_timeline_clip_info(&self, clip_id: i32) -> zbus::Result<HashMap<String, OwnedValue>>;
    #[zbus(name = "scriptGetProjectFps")]
    fn script_get_project_fps(&self) -> zbus::Result<f64>;
    #[zbus(name = "scriptImportMedia")]
    fn script_import_media(&self, file_paths: Vec<String>, folder_id: String) -> zbus::Result<Vec<String>>;
    #[zbus(name = "scriptDeleteTimelineClip")]
    fn script_delete_timeline_clip(&self, clip_id: i32) -> zbus::Result<bool>;
    #[zbus(name = "scriptInsertClip")]
    fn script_insert_clip(&self, bin_clip_id: String, track_id: i32, position: i32) -> zbus::Result<i32>;
}

pub fn connect() -> AppResult<(zbus::blocking::Connection, String)> {
    let conn = zbus::blocking::Connection::session()
        .map_err(|e| AppError::new(format!("session bus: {e}"), "kdenlive::talker::connect", file!()))?;
    let names = list_running(&conn)?;
    match names.len() {
        0 => Err(AppError::new("Kdenlive is not running", "kdenlive::talker::connect", file!())),
        1 => Ok((conn, names.into_iter().next().unwrap())),
        _ => {
            let pids: Vec<&str> = names
                .iter()
                .map(|n| n.strip_prefix("org.kde.kdenlive-").unwrap_or(n.as_str()))
                .collect();
            Err(AppError::new(
                format!(
                    "{} Kdenlive instances running (PIDs: {}); close all but one",
                    names.len(),
                    pids.join(", ")
                ),
                "kdenlive::talker::connect",
                file!(),
            ))
        }
    }
}

pub fn list_running(conn: &zbus::blocking::Connection) -> AppResult<Vec<String>> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .map_err(|e| AppError::new(format!("ListNames: {e}"), "kdenlive::talker::list_running", file!()))?;

    let names: Vec<String> = reply
        .body()
        .deserialize()
        .map_err(|e| AppError::new(format!("parse names: {e}"), "kdenlive::talker::list_running", file!()))?;

    let mut kd: Vec<(u32, String)> = names
        .into_iter()
        .filter_map(|n| {
            n.strip_prefix("org.kde.kdenlive-")
                .and_then(|s| s.parse::<u32>().ok())
                .map(|pid| (pid, n))
        })
        .collect();

    kd.sort_by_key(|(pid, _)| *pid);
    Ok(kd.into_iter().map(|(_, n)| n).collect())
}

pub fn check_status() -> KdenliveStatus {
    let conn = match zbus::blocking::Connection::session() {
        Ok(c) => c,
        Err(e) => return KdenliveStatus::Error(format!("session bus: {e}")),
    };
    match list_running(&conn) {
        Ok(names) => match names.len() {
            0 => KdenliveStatus::NotRunning,
            1 => KdenliveStatus::Connected { name: names.into_iter().next().unwrap() },
            _ => KdenliveStatus::Multiple { count: names.len(), names },
        },
        Err(e) => KdenliveStatus::Error(e.to_string()),
    }
}

pub fn build_proxy<'a>(
    conn: &'a zbus::blocking::Connection,
    name: &str,
) -> AppResult<KdenliveMainWindowProxyBlocking<'a>> {
    KdenliveMainWindowProxyBlocking::builder(conn)
        .destination(name.to_string())
        .map_err(|e| AppError::new(format!("destination: {e}"), "kdenlive::talker::build", file!()))?
        .path("/MainWindow")
        .map_err(|e| AppError::new(format!("path: {e}"), "kdenlive::talker::build", file!()))?
        .build()
        .map_err(|e| AppError::new(format!("build: {e}"), "kdenlive::talker::build", file!()))
}
