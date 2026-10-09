use std::path::PathBuf;
use std::process::{Command, Stdio};

const HOST_ID: &str = "zapgui";
const POPUP_W: f32 = 360.0;
const POPUP_H: f32 = 100.0;

pub fn show(text: &str) {
    show_with_size(text, None);
}

pub fn show_with_size(text: &str, screen_size: Option<(f32, f32)>) {
    let Some(binary) = find_binary() else { return };
    let (w, h) = screen_size.unwrap_or((1920.0, 1080.0));
    let x = (w / 2.0 - POPUP_W / 2.0).round() as i32;
    let y = (h - POPUP_H - 40.0).round() as i32;
    if let Ok(mut child) = Command::new(&binary)
        .arg(text)
        .arg(x.to_string())
        .arg(y.to_string())
        .arg(HOST_ID)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

fn find_binary() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let local = PathBuf::from(&home).join(".local/bin/popup_demo_app");
    if local.exists() {
        return Some(local);
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            if dir.is_empty() {
                continue;
            }
            let candidate = PathBuf::from(dir).join("popup_demo_app");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
