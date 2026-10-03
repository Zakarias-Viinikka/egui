use crate::text_diff::{pair_lines, process};

pub fn route(request: &str) -> String {
    if request == "replace" || request.starts_with("replace ") {
        focus_window();
        return String::new();
    }
    if let Some(rest) = request.strip_prefix("overwrite ") {
        return handle_overwrite(rest);
    }
    format!("zapgui: got {request:?}")
}

/// Payload shape: `<path>\n<content>`. The path is on the first line, and
/// everything after that first newline is the new file body verbatim.
fn handle_overwrite(rest: &str) -> String {
    let Some(nl) = rest.find('\n') else {
        return "zapgui: overwrite needs a newline between path and content".to_string();
    };
    let path = rest[..nl].trim();
    let content = &rest[nl + 1..];

    let old = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return format!("zapgui: couldn't find {path}"),
    };

    let rows = process(&pair_lines(&old, content));
    super::stash_diff(rows);
    String::new()
}

/// Bring the zapgui window to the front.
///
/// ViewportCommand::Focus only sets keyboard focus, it doesn't raise the
/// window on XFCE. wmctrl -a does both. This is called from the UI thread,
/// after it has made the window visible — calling wmctrl on a hidden window
/// may silently do nothing.
pub fn focus_window() {
    let _ = std::process::Command::new("wmctrl")
        .args(["-a", "zapgui"])
        .spawn();
}
