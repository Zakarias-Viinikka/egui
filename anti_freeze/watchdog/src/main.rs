use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

fn state_char(pid: u32) -> Result<Option<char>, String> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).map_err(|e| {
        format!("err_msg: read /proc/{pid}/status failed: {e} | method: state_char | file: watchdog/src/main.rs")
    })?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("State:") {
            return Ok(rest.trim_start().chars().next());
        }
    }
    Ok(None)
}

fn parent_alive(pid: u32) -> bool {
    if !Path::new(&format!("/proc/{pid}")).exists() {
        return false;
    }
    match state_char(pid) {
        Ok(Some('Z')) => false,
        Ok(_) => true,
        // a read failing on a pid that was just there means it is gone
        Err(e) => {
            eprintln!("{e}");
            false
        }
    }
}

fn unfreeze_from_file(path: &Path) -> Result<(), String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(format!(
                "err_msg: read {} failed: {e} | method: unfreeze_from_file | file: watchdog/src/main.rs",
                path.display()
            ));
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let pid: u32 = match line.parse() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("err_msg: bad pid {line:?}: {e} | method: unfreeze_from_file | file: watchdog/src/main.rs");
                continue;
            }
        };
        match state_char(pid) {
            Ok(Some('T')) | Ok(Some('t')) => {}
            Ok(_) => continue,
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        }
        let output = match Command::new("kill").arg("-CONT").arg(pid.to_string()).output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("err_msg: spawn kill -CONT failed for pid {pid}: {e} | method: unfreeze_from_file | file: watchdog/src/main.rs");
                continue;
            }
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            eprintln!("err_msg: kill -CONT {pid} failed: {stderr} | method: unfreeze_from_file | file: watchdog/src/main.rs");
        }
    }
    Ok(())
}

fn show_popup(msg: &str) {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("err_msg: current_exe failed: {e} | method: show_popup | file: watchdog/src/main.rs");
            return;
        }
    };
    let dir = match exe.parent() {
        Some(d) => d.to_path_buf(),
        None => {
            eprintln!("err_msg: current_exe has no parent | method: show_popup | file: watchdog/src/main.rs");
            return;
        }
    };
    let bin = dir.join("fading_popup");
    if !bin.exists() {
        eprintln!("err_msg: fading_popup not found at {} | method: show_popup | file: watchdog/src/main.rs", bin.display());
        return;
    }
    if let Err(e) = std::process::Command::new(&bin)
        .arg(msg)
        .arg("100")
        .arg("100")
        .spawn()
    {
        eprintln!("err_msg: spawn fading_popup failed: {e} | method: show_popup | file: watchdog/src/main.rs");
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let parent_arg = match args.next() {
        Some(a) => a,
        None => {
            eprintln!("err_msg: usage: watchdog <parent_pid> <frozen_file> | method: main | file: watchdog/src/main.rs");
            return;
        }
    };
    let parent: u32 = match parent_arg.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("err_msg: bad parent pid {parent_arg:?}: {e} | method: main | file: watchdog/src/main.rs");
            return;
        }
    };
    let file = match args.next() {
        Some(a) => PathBuf::from(a),
        None => {
            eprintln!("err_msg: usage: watchdog <parent_pid> <frozen_file> | method: main | file: watchdog/src/main.rs");
            return;
        }
    };

    while parent_alive(parent) {
        std::thread::sleep(Duration::from_millis(500));
    }

    if let Err(e) = unfreeze_from_file(&file) {
        eprintln!("{e}");
    }
    show_popup("Watchdog: unfroze");
    if let Err(e) = std::fs::remove_file(&file) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("err_msg: remove {} failed: {e} | method: main | file: watchdog/src/main.rs", file.display());
        }
    }
}
