use crate::chain::{play_forward, Step, Verb};
use crate::text_diff::{pair_lines, process};

pub fn route(request: &str) -> String {
    if request == "focus" || request.starts_with("focus ") {
        focus_window();
        return String::new();
    }
    if request == "menu" {
        super::request_menu();
        return String::new();
    }
    if let Some(rest) = request.strip_prefix("chain ") {
        return handle_chain(rest);
    }
    if let Some(rest) = request.strip_prefix("overwrite ") {
        return handle_overwrite(rest);
    }
    format!("zapgui: got {request:?}")
}

/// Payload shape: `<cwd>\n<chain block>`. The first line is zapcli's
/// working directory, used to resolve relative paths inside the block.
fn handle_chain(rest: &str) -> String {
    let Some(nl) = rest.find('\n') else {
        return "zapgui: chain needs a newline after cwd".to_string();
    };
    let cwd = rest[..nl].trim();
    let block = &rest[nl + 1..];

    let steps = match crate::chain::parse(block) {
        Ok(s) => s,
        Err(e) => {
            let sentence = format!("zap: nothing was written. {}.", e);
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            crate::chain::popup::show("Edit failed");
            return sentence;
        }
    };

    if steps.is_empty() {
        return "zapgui: empty chain".to_string();
    }

    let chain = match play_forward(steps, std::path::Path::new(cwd)) {
        Ok(c) => c,
        Err(e) => {
            let sentence = format!("zap: nothing was written. {}.", e);
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            crate::chain::popup::show("Edit failed");
            return sentence;
        }
    };

    // If every touched file lives inside a whitelisted folder, apply
    // immediately. No diff, no window. A chain with even one file outside
    // goes wholly to review, because applying part of a chain would break
    // all-or-nothing.
    let folders = crate::chain::whitelist::load();
    let all_whitelisted = !folders.is_empty()
        && chain
            .snapshots
            .iter()
            .all(|s| crate::chain::whitelist::is_inside(&s.path, &folders));
    if all_whitelisted {
        let wanted = vec![true; chain.steps.len()];
        return match crate::chain::writer::apply(&chain, &wanted) {
            Ok(()) => {
                let statuses = vec![crate::chain::StepStatus::Applied; chain.steps.len()];
                let rep = crate::chain::report::build(&chain, &statuses, None);
                let _ = crate::clipboard::copy_to_clipboard(&rep);
                crate::chain::popup::show("Change was made");
                String::new()
            }
            Err(e) => {
                let sentence = format!("zap: nothing was written. {}.", e);
                let _ = crate::clipboard::copy_to_clipboard(&sentence);
                crate::chain::popup::show("Edit failed");
                sentence
            }
        };
    }

    match super::stash_pending_chain(chain) {
        Some(displaced) => {
            let first = displaced
                .steps
                .first()
                .map(|s| s.path.display().to_string())
                .unwrap_or_default();
            let sentence = format!("zap: replaced unconfirmed chain ({first}).");
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            crate::chain::popup::show("Replaced");
            sentence
        }
        None => String::new(),
    }
}

/// Payload shape: `<path>\n<content>`. The path is on the first line, and
/// everything after that first newline is the new file body verbatim.
/// Kept working as a one-step overwrite chain.
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

    let step = Step {
        verb: Verb::Overwrite,
        path: path.to_string(),
        old_text: String::new(),
        new_text: normalized,
        count: None,
    };

    let chain = match play_forward(vec![step], std::path::Path::new("/")) {
        Ok(c) => c,
        Err(e) => {
            let sentence = format!("zap: nothing was written. {}.", e);
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            crate::chain::popup::show("Edit failed");
            return sentence;
        }
    };

    match super::stash_pending_chain(chain) {
        Some(_) => String::new(),
        None => String::new(),
    }
}


/// Match the trailing-newline count of `old` in `new`, unless `new` is empty.
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

/// Bring the zapgui window to the front. wmctrl on a hidden window silently
/// does nothing, so callers must make it visible first.
pub fn focus_window() {
    let _ = std::process::Command::new("wmctrl")
        .args(["-a", "zapgui"])
        .spawn();
}

// Unused placeholder so the module still compiles while the old
// PendingOverwrite type sticks around.
#[allow(dead_code)]
fn _unused() {
    let _ = process(&pair_lines("", ""));
}
