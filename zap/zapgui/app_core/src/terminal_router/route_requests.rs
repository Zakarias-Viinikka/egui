use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use zap_wire::{read_frame, write_frame, Frame, FrameKind};

use crate::chain::{play_forward, Step, Verb};
use crate::text_diff::{pair_lines, process};

/// Streaming requests write their own frames and return true. A false
/// return means the caller should send a normal Reply. Only the dev-only
/// `devstream` is streamed for now; the cargo verb lands here later.
pub fn try_stream(request: &str, stream: &mut UnixStream) -> bool {
    if request == "devstream" {
        stream_devstream(stream);
        return true;
    }
    false
}

/// Handle a CargoRun frame directly. Streams its own frames and never
/// produces a Reply.
pub fn handle_cargo_run(stream: &mut UnixStream, text: &str) {
    let Some((cwd, args)) = zap_wire::Frame::decode_cargo_run(text) else {
        let _ = write_frame(stream, &Frame::stream_open("cargo"));
        let _ = write_frame(stream, &Frame::stream_data("zapgui: bad cargo frame\n"));
        let _ = write_frame(stream, &Frame::stream_done("bad frame"));
        return;
    };
    if let Err(e) = crate::terminal_router::cargo_run::run(
        stream,
        args,
        std::path::PathBuf::from(cwd),
    ) {
        let _ = write_frame(stream, &Frame::stream_data(&format!("{e}\n")));
        let _ = write_frame(stream, &Frame::stream_done("error"));
    }
}

/// Handle a RunFile frame. Same shape as cargo: streams its own frames.
pub fn handle_run_file(stream: &mut UnixStream, text: &str) {
    let Some((cwd, args)) = zap_wire::Frame::decode_run_file(text) else {
        let _ = write_frame(stream, &Frame::stream_open("run"));
        let _ = write_frame(stream, &Frame::stream_data("zapgui: bad run frame\n"));
        let _ = write_frame(stream, &Frame::stream_done("bad frame"));
        return;
    };
    if let Err(e) = crate::terminal_router::run_file::run(
        stream,
        args,
        std::path::PathBuf::from(cwd),
    ) {
        let _ = write_frame(stream, &Frame::stream_data(&format!("{e}\n")));
        let _ = write_frame(stream, &Frame::stream_done("error"));
    }
}

fn stream_devstream(stream: &mut UnixStream) {
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut reader) = stream.try_clone() {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            while let Ok(f) = read_frame(&mut reader) {
                if f.kind == FrameKind::Cancel {
                    cancel.store(true, Ordering::SeqCst);
                    break;
                }
            }
        });
    }

    let _ = write_frame(stream, &Frame::stream_open("devstream"));
    for i in 1..=10 {
        if cancel.load(Ordering::SeqCst) {
            let _ = write_frame(stream, &Frame::stream_done("cancelled"));
            return;
        }
        let _ = write_frame(
            stream,
            &Frame::stream_data(format!("devstream line {i}\n")),
        );
        std::thread::sleep(Duration::from_millis(220));
    }
    let _ = write_frame(stream, &Frame::stream_done("exit 0"));
}

pub fn route(request: &str) -> String {
    if request == "focus" || request.starts_with("focus ") {
        focus_window();
        return String::new();
    }
    if request == "menu" {
        super::request_menu();
        return String::new();
    }
    if request == "last" {
        return handle_last(None);
    }
    if let Some(rest) = request.strip_prefix("last ") {
        return handle_last(Some(rest.trim()));
    }
    if let Some(rest) = request.strip_prefix("surfacereview ") {
        return handle_surface_review(rest);
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
    // all-or-nothing. A chain with any Remove or Move step always goes to
    // review, so an auto-applied edit can never be paired with an
    // auto-applied delete or rename.
    let has_remove_or_move = chain
        .steps
        .iter()
        .any(|s| matches!(s.step.verb, Verb::Remove | Verb::Move));
    let folders = crate::chain::whitelist::load();
    let all_whitelisted = !has_remove_or_move
        && !folders.is_empty()
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
        dest: None,
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

    // The bare overwrite form is a one-step chain, so it uses the same
    // whitelist path a chain does.
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
                let statuses =
                    vec![crate::chain::StepStatus::Applied; chain.steps.len()];
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

    super::stash_pending_chain(chain);
    String::new()
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


/// Return a text reply showing the last change zapgui made to a file.
/// `path_opt` is None for "the most recently changed file".
fn handle_last(path_opt: Option<&str>) -> String {
    use crate::last_change::{self, Before, Kind};

    let (record, path) = match path_opt {
        Some(p) => {
            let path = std::path::PathBuf::from(p);
            match last_change::lookup(&path) {
                Some(r) => (r, path),
                None => {
                    return "no recorded change; only changes zap wrote since zapgui started are kept"
                        .to_string();
                }
            }
        }
        None => {
            let Some(path) = last_change::most_recent() else {
                return "no recorded change; only changes zap wrote since zapgui started are kept"
                    .to_string();
            };
            match last_change::lookup(&path) {
                Some(r) => (r, path),
                None => {
                    return "no recorded change; only changes zap wrote since zapgui started are kept"
                        .to_string();
                }
            }
        }
    };

    let mut out = format!("{} ({})", path.display(), format_age(record.at));
    out.push('\n');

    let before_text = match &record.before {
        Before::Text(s) => s.clone(),
        Before::Missing => String::new(),
        Before::Skipped(n) => {
            out.push_str(&format!("file too large or binary ({n} bytes); no diff\n"));
            return out;
        }
    };

    let after_text = match &record.kind {
        Kind::Removed => String::new(),
        _ => std::fs::read_to_string(&path).unwrap_or_default(),
    };

    if let Some(expected) = record.after_hash {
        if last_change::hash_text(&after_text) != expected {
            out.push_str(
                "note: the file changed after zap wrote it; this diff includes those changes\n",
            );
        }
    }

    let diff = similar::TextDiff::from_lines(&before_text, &after_text);
    let s = diff.unified_diff().context_radius(3).to_string();
    let mut count = 0;
    for line in s.lines() {
        if count >= 400 {
            out.push_str("... truncated at 400 lines\n");
            break;
        }
        out.push_str(line);
        out.push('\n');
        count += 1;
    }
    out
}

fn format_age(t: std::time::SystemTime) -> String {
    let secs = t.elapsed().map(|d| d.as_secs()).unwrap_or(0);
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

/// Dev request: collect the surface for a path, compare it against the
/// stored trust record, and stash a review if there are diffs. Blocks
/// until the user confirms or denies. On confirm, stores the new surface
/// as trusted. On deny, does nothing. Once the cargo verb exists this
/// becomes part of the run flow.
fn handle_surface_review(rest: &str) -> String {
    let path = rest.trim();
    if path.is_empty() {
        return "zapgui: surfacereview needs a path".to_string();
    }
    let start = std::path::Path::new(path);
    let Some(root) = crate::surface::project_root(start) else {
        return format!("zapgui: no Cargo.toml at or above {path}");
    };
    let surface = match crate::surface::collect(&root) {
        Ok(s) => s,
        Err(e) => return format!("zapgui: {e}"),
    };
    let diffs: Vec<crate::trust::SurfaceDiff> = match crate::trust::load(&root) {
        Some(record) => crate::trust::compare(&record, &surface),
        None => surface
            .iter()
            .map(|f| crate::trust::SurfaceDiff {
                path: f.path.clone(),
                old_text: None,
                new_text: f.text.clone(),
            })
            .collect(),
    };
    if diffs.is_empty() {
        return "zapgui: surface unchanged".to_string();
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let caption = format!("execution surface changed for {}", root.display());
    let review = crate::terminal_router::surface_review::SurfaceReview::new(
        root.clone(),
        diffs,
        surface,
        tx,
        crate::terminal_router::surface_review::BannerKind::CargoRun,
        caption,
    );
    if !crate::terminal_router::stash_surface_review(review) {
        return "zapgui: another surface review is already showing".to_string();
    }

    let accepted = rx
        .recv_timeout(std::time::Duration::from_secs(600))
        .unwrap_or(false);

    if !accepted {
        return "zapgui: surface review denied".to_string();
    }
    let fresh = match crate::surface::collect(&root) {
        Ok(s) => s,
        Err(e) => return format!("zapgui: {e}"),
    };
    let record = crate::trust::from_surface(&root, &fresh);
    match crate::trust::save(&record) {
        Ok(()) => "zapgui: surface trusted".to_string(),
        Err(e) => format!("zapgui: {e}"),
    }
}
