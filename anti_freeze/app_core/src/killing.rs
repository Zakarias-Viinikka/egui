pub fn kill_pids(pids: &[u32]) -> Result<(), String> {
    for pid in pids {
        let output = std::process::Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()
            .map_err(|e| {
                format!(
                    "err_msg: spawning kill failed for pid {pid}: {e} | method: kill_pids | file: app_core/src/killing.rs"
                )
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!(
                "err_msg: kill -9 {pid} failed: {stderr} | method: kill_pids | file: app_core/src/killing.rs"
            ));
        }
    }
    Ok(())
}

fn our_uid() -> Result<u32, String> {
    let status = std::fs::read_to_string("/proc/self/status").map_err(|e| {
        format!("err_msg: read /proc/self/status failed: {e} | method: our_uid | file: app_core/src/killing.rs")
    })?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            if let Some(first) = rest.split_whitespace().next() {
                return first.parse::<u32>().map_err(|e| {
                    format!("err_msg: parse uid failed: {e} | method: our_uid | file: app_core/src/killing.rs")
                });
            }
        }
    }
    Err(String::from(
        "err_msg: no Uid line in /proc/self/status | method: our_uid | file: app_core/src/killing.rs",
    ))
}

// Processes to never SIGSTOP. Stopping these breaks our ability to unfreeze.
pub const NEVER_STOP: &[&str] = &[
    "Xorg",
    "Xwayland",
    "xfwm4",
    "xfce4-session",
    "xfsettingsd",
    "xfconfd",
    "xfce4-notifyd",
    "xfce4-panel",
    "xfdesktop",
    "lightdm",
    "dbus-daemon",
    "dbus-broker",
    "at-spi-bus-laun",
    "pulseaudio",
    "pipewire",
    "pipewire-pulse",
    "wireplumber",
    "picom",
    "compton",
    "xfce4-composito",
    "watchdog",
];

pub fn freeze_other_processes() -> Result<Vec<u32>, String> {
    let our_pid = std::process::id();
    let our_uid = our_uid()?;
    let mut stopped: Vec<u32> = Vec::new();

    let entries = std::fs::read_dir("/proc").map_err(|e| {
        format!("err_msg: read_dir /proc failed: {e} | method: freeze_other_processes | file: app_core/src/killing.rs")
    })?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let pid_str = entry.file_name().to_string_lossy().to_string();
        let pid: u32 = match pid_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if pid == our_pid {
            continue;
        }

        let status = match std::fs::read_to_string(format!("/proc/{}/status", pid)) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut proc_uid: Option<u32> = None;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(first) = rest.split_whitespace().next() {
                    proc_uid = first.parse().ok();
                }
                break;
            }
        }
        if proc_uid != Some(our_uid) {
            continue;
        }

        let comm = std::fs::read_to_string(format!("/proc/{}/comm", pid))
            .unwrap_or_default();
        let comm = comm.trim();
        if NEVER_STOP.contains(&comm) {
            continue;
        }

        let output = std::process::Command::new("kill")
            .arg("-STOP")
            .arg(pid.to_string())
            .output()
            .map_err(|e| {
                format!("err_msg: spawn kill -STOP failed for pid {pid}: {e} | method: freeze_other_processes | file: app_core/src/killing.rs")
            })?;
        if output.status.success() {
            stopped.push(pid);
        }
    }
    Ok(stopped)
}

pub fn unfreeze_pids(pids: &[u32]) -> Result<(), String> {
    let mut failed: Vec<u32> = Vec::new();
    for pid in pids {
        if !std::path::Path::new(&format!("/proc/{}", pid)).exists() {
            continue;
        }
        let output = std::process::Command::new("kill")
            .arg("-CONT")
            .arg(pid.to_string())
            .output()
            .map_err(|e| {
                format!("err_msg: spawn kill -CONT failed for pid {pid}: {e} | method: unfreeze_pids | file: app_core/src/killing.rs")
            })?;
        if !output.status.success() {
            failed.push(*pid);
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "err_msg: failed to CONT pids {:?} | method: unfreeze_pids | file: app_core/src/killing.rs",
            failed
        ))
    }
}
