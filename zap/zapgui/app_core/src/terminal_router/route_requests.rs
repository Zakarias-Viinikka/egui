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
        None,
        false,
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
        None,
        false,
    ) {
        let _ = write_frame(stream, &Frame::stream_data(&format!("{e}\n")));
        let _ = write_frame(stream, &Frame::stream_done("error"));
    }
}


/// Handle a ChainRun frame. Runs on the connection's own thread and blocks
/// for the life of the chain: parse, play forward, stash for review, wait
/// for the UI's outcome, then dispatch the run step (if wanted) with the
/// same cancel flag the watcher uses.
pub fn handle_chain_run(stream: &mut UnixStream, text: &str) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;

    // One watcher. The run below uses its flag instead of spawning a
    // second reader on the same socket.
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut reader) = stream.try_clone() {
        let cf = cancel.clone();
        std::thread::spawn(move || loop {
            match read_frame(&mut reader) {
                Ok(f) if f.kind == FrameKind::Cancel => {
                    cf.store(true, Ordering::SeqCst);
                    break;
                }
                Ok(_) => {}
                Err(_) => {
                    cf.store(true, Ordering::SeqCst);
                    break;
                }
            }
        });
    }

    let Some(nl) = text.find('\n') else {
        let _ = write_frame(stream, &Frame::reply("zapgui: chain-run needs a newline after cwd"));
        return;
    };
    let cwd = text[..nl].trim();
    let block = &text[nl + 1..];

    let steps = match crate::chain::parse(block) {
        Ok(s) => s,
        Err(e) => {
            let sentence = format!("zap: nothing was written. {}.", e);
            crate::chain::popup::show("Edit failed");
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            let _ = write_frame(stream, &Frame::reply(&sentence));
            return;
        }
    };
    if !steps.iter().any(|s| s.verb.is_run()) {
        let _ = write_frame(stream, &Frame::reply("zapgui: not a run chain"));
        return;
    }

    let mut chain = match play_forward(steps.clone(), std::path::Path::new(cwd)) {
        Ok(c) => c,
        Err(e) => {
            let sentence = format!("zap: nothing was written. {}.", e);
            let statuses = vec![crate::chain::StepStatus::Failed; steps.len()];
            crate::terminal_router::retry::save_from_steps(steps, statuses);
            crate::chain::popup::show("Edit failed");
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            let _ = write_frame(stream, &Frame::reply(&sentence));
            return;
        }
    };

    // Partial approval: only the steps that need eyes are shown. A run
    // chain always has at least the run step in the shown set.
    let folders = crate::chain::whitelist::load();
    let needing = crate::chain::play::steps_that_need_eyes(
        &chain,
        &folders,
        |p, text| {
            crate::terminal_router::file_trust::is_sensitive(p)
                && !crate::terminal_router::file_trust::is_approved(p, text)
        },
    );
    if needing.len() < chain.steps.len() {
        chain.review_indices = needing;
    }

    let (tx, rx) = mpsc::channel();
    chain.notify = Some(std::sync::Mutex::new(tx));
    let mut mine = Arc::new(chain);

    // Fast path: a single run or cargo step whose target is already
    // trusted for this exact invocation. Skip the chain review and run
    // it. The run handlers (run_file::run / cargo_run::run) re-check
    // trust on their own and skip their own review when it matches, so
    // a fresh target produces no window at all; a stale one falls back
    // to the run handler's review. See TRUST_REUSE_NOTES.md.
    if let Some((verb, folder, args)) = run_if_trusted(&mine) {
        let res = match verb {
            Verb::Cargo => crate::terminal_router::cargo_run::run(
                stream,
                args,
                folder,
                Some(cancel.clone()),
                false,
            ),
            Verb::Run => crate::terminal_router::run_file::run(
                stream,
                args,
                folder,
                Some(cancel.clone()),
                false,
            ),
            _ => unreachable!(),
        };
        if let Err(e) = res {
            let _ = write_frame(stream, &Frame::stream_data(&format!("{e}\n")));
            let _ = write_frame(stream, &Frame::stream_done("error"));
        }
        return;
    }

    if let Some(displaced) = crate::terminal_router::stash_pending_arc(mine.clone()) {
        displaced.finish(crate::chain::ChainOutcome::Replaced);
        let first = displaced
            .steps
            .first()
            .map(|s| s.path.display().to_string())
            .unwrap_or_default();
        let sentence = format!("zap: replaced unconfirmed chain ({first}).");
        crate::chain::popup::show("Replaced");
        let _ = crate::clipboard::copy_to_clipboard(&sentence);
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(600);
    let outcome = loop {
        if cancel.load(Ordering::SeqCst) {
            drain_superseded(&rx, &mut mine);
            crate::terminal_router::clear_pending(&mine);
            let _ = write_frame(stream, &Frame::reply("zap: cancelled"));
            return;
        }
        if std::time::Instant::now() >= deadline {
            drain_superseded(&rx, &mut mine);
            crate::terminal_router::clear_pending(&mine);
            let _ = write_frame(stream, &Frame::reply("zap: timed out"));
            return;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(crate::chain::ChainOutcome::Superseded(next)) => {
                mine = next.0;
                continue;
            }
            Ok(o) => break o,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                drain_superseded(&rx, &mut mine);
                let _ = write_frame(stream, &Frame::reply("zap: chain handler closed"));
                return;
            }
        }
    };

    match outcome {
        crate::chain::ChainOutcome::Applied(wanted) => {
            let run_idx = mine
                .steps
                .iter()
                .position(|s| matches!(s.step.verb, Verb::Cargo | Verb::Run));
            let Some(ri) = run_idx else {
                let _ = write_frame(stream, &Frame::reply("zap: no run step"));
                return;
            };
            if !wanted.get(ri).copied().unwrap_or(false) {
                let _ = write_frame(stream, &Frame::reply("zap: applied, run denied"));
                return;
            }
            let step = &mine.steps[ri];
            let folder = step.path.clone();
            let args = step.step.args.clone();
            let res = match step.step.verb {
                Verb::Cargo => crate::terminal_router::cargo_run::run(
                    stream,
                    args,
                    folder,
                    Some(cancel.clone()),
                    true,
                ),
                Verb::Run => crate::terminal_router::run_file::run(
                    stream,
                    args,
                    folder,
                    Some(cancel.clone()),
                    true,
                ),
                _ => unreachable!(),
            };
            if let Err(e) = res {
                let _ = write_frame(stream, &Frame::stream_data(&format!("{e}\n")));
                let _ = write_frame(stream, &Frame::stream_done("error"));
            }
        }
        crate::chain::ChainOutcome::AllDenied => {
            let _ = write_frame(stream, &Frame::reply("zap: all steps denied"));
        }
        crate::chain::ChainOutcome::Replaced => {
            let _ = write_frame(stream, &Frame::reply("zap: this chain was replaced"));
        }
        crate::chain::ChainOutcome::Superseded(_) => {
            // Only reachable if the very first outcome was Superseded,
            // which cannot happen: the UI always sends Superseded after
            // the swap, so the wait loop reassigns mine and continues.
            let _ = write_frame(stream, &Frame::reply("zap: chain was edited"));
        }
    }
}

/// Drain any Superseded messages that arrived while we were waiting.
/// Each one replaces `mine` with the newer chain, so clear_pending acts
/// on the live chain rather than the one the reviewer replaced.
fn drain_superseded(
    rx: &std::sync::mpsc::Receiver<crate::chain::ChainOutcome>,
    mine: &mut Arc<crate::chain::Chain>,
) {
    while let Ok(o) = rx.try_recv() {
        if let crate::chain::ChainOutcome::Superseded(next) = o {
            *mine = next.0;
        }
    }
}

/// Resolve a user-typed target the way run_file::run does, but without
/// the canonicalize step (the caller canonicalizes).
fn resolve_target_like_run_file(raw: &str, folder: &std::path::Path) -> Option<std::path::PathBuf> {
    if raw == "~" {
        return std::env::var("HOME").ok().map(std::path::PathBuf::from);
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return std::env::var("HOME")
            .ok()
            .map(|h| std::path::PathBuf::from(h).join(rest));
    }
    let p = std::path::PathBuf::from(raw);
    if p.is_absolute() {
        Some(p)
    } else {
        Some(folder.join(p))
    }
}

/// If the chain is exactly one Run or Cargo step whose target is already
/// trusted for this invocation, return (verb, folder, args). Otherwise
/// None. See TRUST_REUSE_NOTES.md for the caveats.
fn run_if_trusted(
    chain: &crate::chain::Chain,
) -> Option<(Verb, std::path::PathBuf, Vec<String>)> {
    if chain.steps.len() != 1 {
        return None;
    }
    let step = &chain.steps[0];
    match step.step.verb {
        Verb::Run => {
            if step.step.args.is_empty() {
                return None;
            }
            let folder = step.path.clone();
            let target_raw = &step.step.args[0];
            let extra_args: Vec<String> = step.step.args[1..].to_vec();
            let file = resolve_target_like_run_file(target_raw, &folder)?.canonicalize().ok()?;
            let (sha, _) = crate::run_trust::read_for_entry(&file).ok()?;
            let entry = crate::run_trust::load(&file, &extra_args);
            match crate::run_trust::decide(entry.as_ref(), &sha) {
                crate::run_trust::Decision::Trusted => {
                    Some((Verb::Run, folder, step.step.args.clone()))
                }
                _ => None,
            }
        }
        Verb::Cargo => {
            let folder = step.path.clone();
            let root = crate::surface::project_root(&folder)?;
            let surf = crate::surface::collect(&root).ok()?;
            let rec = crate::trust::load(&root)?;
            if crate::trust::compare(&rec, &surf).is_empty() {
                Some((Verb::Cargo, folder, step.step.args.clone()))
            } else {
                None
            }
        }
        _ => None,
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
    if let Some(text) = request.strip_prefix("copy-error\n") {
        return match crate::clipboard::copy_to_clipboard(text) {
            Ok(()) => "zap: copied error to clipboard.".to_string(),
            Err(e) => format!("zap error: {e}"),
        };
    }
    if request == "error" {
        return match crate::clipboard::retry_pending_error() {
            Ok(()) => "zap: copied the retained clipboard text.".to_string(),
            Err(e) => format!("zap error: {e}"),
        };
    }
    if request == "focus" || request.starts_with("focus ") {
        focus_window();
        return String::new();
    }
    if request == "focus-if-busy" {
        if super::something_waiting() {
            super::request_show();
            return "shown".to_string();
        }
        return "idle".to_string();
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
    if request == "retry" {
        return handle_retry(None, None);
    }
    if let Some(rest) = request.strip_prefix("retry ") {
        let (nums_part, body) = match rest.find('\n') {
            Some(nl) => (&rest[..nl], Some(&rest[nl + 1..])),
            None => (rest, None),
        };
        let mut idx: Vec<usize> = Vec::new();
        for tok in nums_part.split_whitespace() {
            match tok.parse::<usize>() {
                Ok(n) if n >= 1 => idx.push(n - 1),
                Ok(_) => {
                    return format!("zapgui: step numbers are 1-based (got {tok})");
                }
                Err(_) => {
                    return format!("zapgui: bad step number {tok}");
                }
            }
        }
        if idx.is_empty() {
            return "zapgui: retry takes step numbers or nothing".to_string();
        }
        return handle_retry(Some(idx), body);
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
            crate::chain::popup::show("Edit failed");
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            return sentence;
        }
    };

    if steps.is_empty() {
        return "zapgui: empty chain".to_string();
    }
    if steps.iter().any(|s| s.verb.is_run()) {
        return "zapgui: this chain has a run step; send it as a ChainRun frame".to_string();
    }

    let mut chain = match play_forward(steps.clone(), std::path::Path::new(cwd)) {
        Ok(c) => c,
        Err(e) => {
            let sentence = format!("zap: nothing was written. {}.", e);
            let statuses = vec![crate::chain::StepStatus::Failed; steps.len()];
            crate::terminal_router::retry::save_from_steps(steps, statuses);
            crate::chain::popup::show("Edit failed");
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            return sentence;
        }
    };

    // Partial approval: split the chain into "needs eyes" and "does not".
    // An empty result means nothing needs eyes and the chain auto-applies;
    // a result that holds every step means everything needs eyes and the
    // review shows it as before; anything between sets review_indices to
    // the shown subset. Hidden steps still get written with the ones the
    // user confirms.
    let folders = crate::chain::whitelist::load();
    let needing = crate::chain::play::steps_that_need_eyes(
        &chain,
        &folders,
        |p, text| {
            crate::terminal_router::file_trust::is_sensitive(p)
                && !crate::terminal_router::file_trust::is_approved(p, text)
        },
    );
    if needing.is_empty() {
        let wanted = vec![true; chain.steps.len()];
        return match crate::chain::writer::apply(&chain, &wanted) {
            Ok(()) => {
                let statuses = vec![crate::chain::StepStatus::Applied; chain.steps.len()];
                let rep = crate::chain::report::build(&chain, &statuses, None);
                crate::chain::popup::show("Change was made");
                let _ = crate::clipboard::copy_to_clipboard(&rep);
                String::new()
            }
            Err(e) => {
                let sentence = format!("zap: nothing was written. {}.", e);
                let statuses =
                    vec![crate::chain::StepStatus::Failed; chain.steps.len()];
                crate::terminal_router::retry::save_from_chain(&chain, &statuses);
                crate::chain::popup::show("Edit failed");
                let _ = crate::clipboard::copy_to_clipboard(&sentence);
                sentence
            }
        };
    }
    if needing.len() < chain.steps.len() {
        chain.review_indices = needing;
    }

    match super::stash_pending_chain(chain) {
        Some(displaced) => {
            let first = displaced
                .steps
                .first()
                .map(|s| s.path.display().to_string())
                .unwrap_or_default();
            let sentence = format!("zap: replaced unconfirmed chain ({first}).");
            crate::chain::popup::show("Replaced");
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
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

    {
        let allow = zap_wire::extensions::ExtensionAllowlist::load();
        if let Some(msg) = allow.check("overwrite", std::path::Path::new(path)) {
            return format!("zapgui: overwrite refused: {msg}");
        }
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
        no_newline: false,
        args: Vec::new(),
        cwd: None,
    };

    let mut chain = match play_forward(vec![step], std::path::Path::new("/")) {
        Ok(c) => c,
        Err(e) => {
            let sentence = format!("zap: nothing was written. {}.", e);
            crate::chain::popup::show("Edit failed");
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            return sentence;
        }
    };

    // The bare overwrite form is a one-step chain, so it goes through the
    // same "steps that need eyes" test as a full chain.
    let folders = crate::chain::whitelist::load();
    let needing = crate::chain::play::steps_that_need_eyes(
        &chain,
        &folders,
        |p, text| {
            crate::terminal_router::file_trust::is_sensitive(p)
                && !crate::terminal_router::file_trust::is_approved(p, text)
        },
    );
    if needing.is_empty() {
        let wanted = vec![true; chain.steps.len()];
        return match crate::chain::writer::apply(&chain, &wanted) {
            Ok(()) => {
                let statuses =
                    vec![crate::chain::StepStatus::Applied; chain.steps.len()];
                let rep = crate::chain::report::build(&chain, &statuses, None);
                crate::chain::popup::show("Change was made");
                let _ = crate::clipboard::copy_to_clipboard(&rep);
                String::new()
            }
            Err(e) => {
                let sentence = format!("zap: nothing was written. {}.", e);
                crate::chain::popup::show("Edit failed");
                let _ = crate::clipboard::copy_to_clipboard(&sentence);
                sentence
            }
        };
    }
    if needing.len() < chain.steps.len() {
        chain.review_indices = needing;
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


/// Rebuild a chain from the last review's unapplied steps and stash it
/// for review. Writes nothing directly; the user confirms as usual.
/// A retry that failed. `had_body` distinguishes a retry the AI wrote
/// (a pasted chain block: copy it to the clipboard and pop a notice) from
/// a bare `retry 2 5` the human typed (print only, nothing copied). The
/// returned string is what goes on screen, and on the clipboard when the
/// copy happens, so screen and paste-back always agree.
fn retry_failure(sentence: String, had_body: bool) -> String {
    if had_body {
        crate::chain::popup::notify_copied("Retry failed", &sentence);
        sentence
    } else {
        // Strip the leading "zap: " so the terminal line reads like the
        // other zapgui errors and doesn't pretend a chain was attempted.
        let rest = sentence.strip_prefix("zap: ").unwrap_or(&sentence);
        format!("zapgui: {rest}")
    }
}

fn handle_retry(only: Option<Vec<usize>>, body: Option<&str>) -> String {
    let Some(rec) = crate::terminal_router::retry::current() else {
        return "zapgui: nothing to retry".to_string();
    };
    let had_body = body.map(|b| !b.trim().is_empty()).unwrap_or(false);

    // Steps carried over from the record were parsed before any hidden
    // entry that might exist now. Refuse one that has become hidden.
    let hidden = zap_wire::hidden::HiddenSet::load();
    for (i, step) in rec.steps.iter().enumerate() {
        for p in std::iter::once(&step.path).chain(step.dest.as_ref()) {
            if p.is_empty() {
                continue;
            }
            if let Some(msg) = hidden.refusal(std::path::Path::new(p)) {
                return retry_failure(
                    format!(
                        "zap: nothing was written. step {} can't be retried: {}",
                        i + 1,
                        msg
                    ),
                    had_body,
                );
            }
        }
        if step.verb.is_run() {
            if let Some(arg) = step.args.first() {
                if let Some(msg) = hidden.refusal(std::path::Path::new(arg)) {
                    return retry_failure(
                        format!(
                            "zap: nothing was written. step {} can't be retried: {}",
                            i + 1,
                            msg
                        ),
                        had_body,
                    );
                }
            }
        }
    }
    let replacements = match body {
        Some(b) if had_body => match crate::chain::parse(b) {
            Ok(steps) => Some(steps),
            Err(e) => {
                return retry_failure(
                    format!("zap: nothing was written. {}.", e),
                    had_body,
                );
            }
        },
        _ => None,
    };
    let chain = match crate::terminal_router::retry::build_retry_chain(
        &rec,
        only.as_deref(),
        replacements.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => {
            return retry_failure(
                format!("zap: nothing was written. {}.", e),
                had_body,
            );
        }
    };
    match super::stash_pending_chain(chain) {
        Some(displaced) => {
            let first = displaced
                .steps
                .first()
                .map(|s| s.path.display().to_string())
                .unwrap_or_default();
            let sentence = format!("zap: replaced unconfirmed chain ({first}).");
            crate::chain::popup::show("Replaced");
            let _ = crate::clipboard::copy_to_clipboard(&sentence);
            sentence
        }
        None => String::new(),
    }
}

/// Return a text reply showing the last change zapgui made to a file.
/// `path_opt` is None for "the most recently changed file".
fn handle_last(path_opt: Option<&str>) -> String {
    use crate::last_change::{self, Before, Kind};

    let (record, path) = match path_opt {
        Some(p) => {
            let path = std::path::PathBuf::from(p);
            if let Some(msg) = zap_wire::hidden::HiddenSet::check(&path) {
                return msg;
            }
            match last_change::lookup(&path) {
                Some(r) => (r, path),
                None => {
                    return "no recorded change; only changes zap wrote since zapgui started are kept"
                        .to_string();
                }
            }
        }
        None => {
            let hidden = zap_wire::hidden::HiddenSet::load();
            let Some(path) = last_change::most_recent_with(|p| !hidden.is_hidden(p)) else {
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

    let removed = matches!(record.kind, Kind::Removed);
    let after_text = if removed {
        String::new()
    } else {
        std::fs::read_to_string(&path).unwrap_or_default()
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
            out.push_str("... diff truncated at 400 lines\n");
            break;
        }
        out.push_str(line);
        out.push('\n');
        count += 1;
    }

    out.push_str("\n--- current contents ---\n");
    if removed {
        out.push_str("(file was removed)\n");
    } else {
        out.push_str(&after_text);
        if !out.ends_with('\n') {
            out.push('\n');
        }
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
