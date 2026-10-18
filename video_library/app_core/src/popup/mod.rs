use std::path::PathBuf;

const POPUP_BIN: &str = "fading_popup";
const MAX_CHARS: usize = 40;

pub fn spawn_popup(message: &str) {
    let Some(path) = popup_binary_path() else {
        eprintln!("[popup] binary not found next to current_exe");
        return;
    };

    let short = truncate(message, MAX_CHARS);

    match std::process::Command::new(&path).arg(&short).spawn() {
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

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let head: String = text.chars().take(max_chars - 3).collect();
    format!("{head}...")
}
