use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn frozen_file(parent_pid: u32) -> PathBuf {
    PathBuf::from(format!("/tmp/anti_freeze_frozen_{parent_pid}"))
}

/// Launches the separate `watchdog` binary (next to our own exe) in its own
/// process group, so killing the app's group does not kill it. It watches our
/// pid and, when we die, resumes every pid listed in our frozen file.
pub fn spawn_guard() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| {
        format!("err_msg: current_exe failed: {e} | method: spawn_guard | file: app_core/src/watchdog.rs")
    })?;
    let dir = exe
        .parent()
        .ok_or_else(|| {
            String::from("err_msg: current_exe has no parent | method: spawn_guard | file: app_core/src/watchdog.rs")
        })?
        .to_path_buf();
    let bin = dir.join("watchdog");
    if !bin.exists() {
        return Err(format!(
            "err_msg: binary not found at {} | method: spawn_guard | file: app_core/src/watchdog.rs",
            bin.display()
        ));
    }
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/anti_freeze_watchdog.log")
        .map_err(|e| {
            format!("err_msg: open watchdog log failed: {e} | method: spawn_guard | file: app_core/src/watchdog.rs")
        })?;
    let parent = std::process::id();
    Command::new(&bin)
        .arg(parent.to_string())
        .arg(frozen_file(parent))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log))
        .process_group(0)
        .spawn()
        .map_err(|e| {
            format!("err_msg: spawn watchdog failed: {e} | method: spawn_guard | file: app_core/src/watchdog.rs")
        })?;
    Ok(())
}

pub fn record_frozen(pids: &[u32]) -> Result<(), String> {
    let text = pids
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(frozen_file(std::process::id()), text).map_err(|e| {
        format!("err_msg: write frozen file failed: {e} | method: record_frozen | file: app_core/src/watchdog.rs")
    })
}

pub fn clear_frozen() -> Result<(), String> {
    match std::fs::remove_file(frozen_file(std::process::id())) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "err_msg: remove frozen file failed: {e} | method: clear_frozen | file: app_core/src/watchdog.rs"
        )),
    }
}
