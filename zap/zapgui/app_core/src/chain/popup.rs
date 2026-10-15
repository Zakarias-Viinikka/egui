use std::path::PathBuf;
use std::process::{Command, Stdio};

const POPUP_W: f32 = 260.0;
/// How far down from the top edge the popup's top sits, in points. Sits
/// above the review banner so a popup and a review can both be on screen.
const POPUP_TOP_OFFSET: f32 = 12.0;

pub fn show(text: &str) {
    show_with_size(text, None);
}

pub fn show_with_size(text: &str, screen_size: Option<(f32, f32)>) {
    let Some(binary) = find_binary() else { return };
    let (w, _h) = screen_size.unwrap_or_else(detect_screen_size);
    let x = (w / 2.0 - POPUP_W / 2.0).round() as i32;
    let y = POPUP_TOP_OFFSET.round() as i32;
    if let Ok(mut child) = Command::new(&binary)
        .arg(text)
        .arg(x.to_string())
        .arg(y.to_string())
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

/// Ask the window system for the display size once, and cache it. The
/// old code assumed 1920x1080, which put the popup in the wrong place on
/// anything else.
fn detect_screen_size() -> (f32, f32) {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Option<(f32, f32)>> = OnceLock::new();
    let cached = *CACHE.get_or_init(|| {
        let out = Command::new("xdotool")
            .args(["getdisplaygeometry"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut parts = text.split_whitespace();
        let w: f32 = parts.next()?.parse().ok()?;
        let h: f32 = parts.next()?.parse().ok()?;
        Some((w, h))
    });
    cached.unwrap_or((1920.0, 1080.0))
}

fn find_binary() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let local = PathBuf::from(&home).join(".local/bin/fading_popup");
    if local.exists() {
        return Some(local);
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            if dir.is_empty() {
                continue;
            }
            let candidate = PathBuf::from(dir).join("fading_popup");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
