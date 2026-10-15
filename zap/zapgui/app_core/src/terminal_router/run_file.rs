//! The run verb's server side. Resolves one file, checks its trust entry,
//! reviews it if the content changed or it's new, then streams it through
//! the shared loop.

use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use zap_wire::{write_frame, Frame};

use crate::clipboard::{self, RecordKind, WriteMode};
use crate::run_trust::{self, Decision, RunEntry};
use crate::runner::{self, RunSpec};
use crate::terminal_router::stream_run::StreamSpec;
use crate::trust::SurfaceDiff;

const REVIEW_TIMEOUT: Duration = Duration::from_secs(600);

pub fn run(
    stream: &mut UnixStream,
    args: Vec<String>,
    cwd: PathBuf,
    cancel_flag: Option<Arc<AtomicBool>>,
    assume_approved: bool,
) -> Result<(), String> {
    if args.is_empty() {
        return Err("run: missing file".to_string());
    }
    let target_arg = &args[0];
    let extra_args: Vec<String> = args[1..].to_vec();

    let Some(path) = resolve_target(target_arg, &cwd) else {
        let _ = write_frame(stream, &Frame::stream_open("run"));
        let _ = write_frame(stream, &Frame::stream_data("run: no such file\n"));
        let _ = write_frame(stream, &Frame::stream_done("not found"));
        return Ok(());
    };

    let meta = match std::fs::metadata(&path) {
        Ok(m) if m.is_file() => m,
        _ => {
            let _ = write_frame(stream, &Frame::stream_open("run"));
            let _ = write_frame(stream, &Frame::stream_data("run: not a file\n"));
            let _ = write_frame(stream, &Frame::stream_done("not a file"));
            return Ok(());
        }
    };

    let is_exec = {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    };
    let is_sh = path.extension().map(|e| e == "sh").unwrap_or(false);
    let program: String;
    let mut run_args: Vec<String>;
    if is_exec {
        program = path.display().to_string();
        run_args = extra_args.clone();
    } else if is_sh {
        program = "bash".to_string();
        run_args = vec![path.display().to_string()];
        run_args.extend(extra_args.iter().cloned());
    } else {
        let _ = write_frame(stream, &Frame::stream_open("run"));
        let _ = write_frame(stream, &Frame::stream_data("run: not executable\n"));
        let _ = write_frame(stream, &Frame::stream_done("not executable"));
        return Ok(());
    }

    let (sha, text) = match run_trust::read_for_entry(&path) {
        Ok(v) => v,
        Err(e) => {
            let _ = write_frame(stream, &Frame::stream_open("run"));
            let _ = write_frame(stream, &Frame::stream_data(&format!("{e}\n")));
            let _ = write_frame(stream, &Frame::stream_done("read error"));
            return Ok(());
        }
    };

    let existing = run_trust::load(&path, &extra_args);
    let decision = run_trust::decide(existing.as_ref(), &sha);
    if let Decision::Review { old_text } = decision {
        let shown_text = if text.is_empty() {
            // Binary or too big: describe instead of showing.
            describe_binary(&path, meta.len(), &sha)
        } else {
            text.clone()
        };
        let diff = SurfaceDiff {
            path: path.clone(),
            old_text,
            new_text: shown_text,
        };
        let first_time = existing.is_none();
        let caption = if first_time {
            format!(
                "execute {} {} \u{00b7} cwd {} \u{00b7} sha256 {} \u{00b7} first time",
                path.display(),
                extra_args.join(" "),
                cwd.display(),
                &sha[..12.min(sha.len())]
            )
        } else {
            format!(
                "execute {} {} \u{00b7} cwd {} \u{00b7} sha256 {} \u{00b7} changed since you approved it",
                path.display(),
                extra_args.join(" "),
                cwd.display(),
                &sha[..12.min(sha.len())]
            )
        };

        // When the run came in on a ChainRun frame, the chain review has
        // already shown this step to the user and gotten one confirm. Skip
        // the second prompt and just record the entry as trusted, so the
        // same content won't re-prompt next time.
        if !assume_approved {
            let (tx, rx) = std::sync::mpsc::channel();
            let review = crate::terminal_router::surface_review::SurfaceReview::new(
                path.clone(),
                vec![diff],
                Vec::new(),
                tx,
                crate::terminal_router::surface_review::BannerKind::Execute,
                caption,
            );
            if !crate::terminal_router::stash_surface_review(review) {
                let _ = write_frame(stream, &Frame::stream_open("run"));
                let _ = write_frame(
                    stream,
                    &Frame::stream_data("run: another surface review is showing\n"),
                );
                let _ = write_frame(stream, &Frame::stream_done("busy"));
                return Ok(());
            }
            let accepted = rx.recv_timeout(REVIEW_TIMEOUT).unwrap_or(false);
            if !accepted {
                let _ = write_frame(stream, &Frame::stream_open("run"));
                let _ = write_frame(stream, &Frame::stream_data("run: denied\n"));
                let _ = write_frame(stream, &Frame::stream_done("denied"));
                crate::chain::popup::show("Run denied");
                let _ = clipboard::write(
                    &format!("zap: run denied for {}", path.display()),
                    RecordKind::Report,
                    WriteMode::Replace,
                );
                return Ok(());
            }
        }

        // Re-read: the file must not have changed while the window was up.
        let (fresh_sha, _) = run_trust::read_for_entry(&path)?;
        if fresh_sha != sha {
            let _ = write_frame(stream, &Frame::stream_open("run"));
            let _ = write_frame(
                stream,
                &Frame::stream_data("run: the file changed while the review was open\n"),
            );
            let _ = write_frame(stream, &Frame::stream_done("changed"));
            return Ok(());
        }

        let entry = RunEntry {
            path: path.clone(),
            args: extra_args.clone(),
            sha: sha.clone(),
            text,
            at_secs: run_trust::now_secs(),
        };
        run_trust::save(&entry)?;
    }

    let title = format!("run {}", path.display());
    let spec = StreamSpec {
        run: RunSpec {
            program,
            args: run_args,
            cwd: cwd.clone(),
            env: runner::pane_env(),
        },
        title,
        summary_prefix: format!("$ run {} {} (in {}, ", path.display(), extra_args.join(" "), cwd.display()),
        popup_ok: "Run: ok".to_string(),
        popup_fail: "Run failed".to_string(),
        after: None,
        cancel_flag,
    };
    crate::terminal_router::stream_run::stream(stream, spec)
}

fn resolve_target(arg: &str, cwd: &Path) -> Option<PathBuf> {
    let raw = if arg == "~" {
        std::env::var("HOME").map(PathBuf::from).ok()?
    } else if let Some(rest) = arg.strip_prefix("~/") {
        let home = std::env::var("HOME").ok()?;
        PathBuf::from(home).join(rest)
    } else {
        let p = PathBuf::from(arg);
        if p.is_absolute() { p } else { cwd.join(p) }
    };
    raw.canonicalize().ok()
}

fn describe_binary(path: &Path, size: u64, sha: &str) -> String {
    format!(
        "binary file, size: {} bytes, sha256: {}\n",
        size,
        &sha[..16.min(sha.len())]
    ) + &format!("path: {}\n", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_binary_starts_with_binary() {
        let s = describe_binary(Path::new("/x"), 42, "abc");
        assert!(s.starts_with("binary file"));
    }
}
