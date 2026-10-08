use crate::terminal_router::pending_overwrite::PendingOverwrite;
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

    if !path.starts_with('/') {
        return format!("zapgui: overwrite needs an absolute path: {path}");
    }

    let old = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return format!("zapgui: couldn't find {path}"),
    };

    let normalized = normalize_newlines(content, &old);
    if normalized == old {
        return format!("zapgui: no change to {path}");
    }

    let rows = process(&pair_lines(&old, &normalized));

    let edit = PendingOverwrite {
        path: path.to_string(),
        old_content: old,
        new_content: normalized,
        rows,
    };

    match super::stash_pending(edit) {
        Some(displaced_path) => format!("zapgui: replaced unconfirmed edit to {displaced_path}"),
        None => String::new(),
    }
}

/// Match the trailing-newline count of `old` in `new`, unless `new` is empty.
/// The listener trims trailing whitespace off the whole request, so the new
/// content arrives without its final newline. Without this, every write would
/// silently strip the last newline off the file.
///
/// "Newline" here is one `\n` character. pair_lines splits on `\n` too, so
/// this matches how the workspace treats lines everywhere.
fn normalize_newlines(new: &str, old: &str) -> String {
    if new.is_empty() {
        return String::new();
    }
    let trailing = old.chars().rev().take_while(|c| *c == '\n').count();
    let mut out = new.trim_end_matches('\n').to_string();
    for _ in 0..trailing {
        out.push('\n');
    }
    out
}

/// Bring the zapgui window to the front.
///
/// ViewportCommand::Focus only sets keyboard focus, it doesn't raise the
/// window on XFCE. wmctrl -a does both. Called from the UI thread after it
/// has made the window visible — wmctrl on a hidden window silently does
/// nothing.
pub fn focus_window() {
    let _ = std::process::Command::new("wmctrl")
        .args(["-a", "zapgui"])
        .spawn();
}
