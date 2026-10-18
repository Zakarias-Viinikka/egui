use std::path::PathBuf;

const POPUP_BIN: &str = "fading_popup";

pub fn spawn_popup(message: &str) {
    let Some(path) = popup_binary_path() else {
        eprintln!("[popup] binary not found next to current_exe");
        return;
    };

    match std::process::Command::new(&path).arg(message).spawn() {
        Ok(_) => {}
        Err(e) => eprintln!("[popup] spawn failed: {e}"),
    }
}

fn popup_binary_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join(POPUP_BIN);
    if candidate.exists() {
        return Some(candidate);
    }
    None
}