//! The cargo verb's server side. Surface check, optional review, then the
//! shared stream loop.

use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use zap_wire::{write_frame, Frame};

use crate::clipboard::{self, RecordKind, WriteMode};
use crate::runner::{self, RunSpec};
use crate::surface;
use crate::terminal_router::stream_run::StreamSpec;
use crate::trust::{self, SurfaceDiff};

const SURFACE_REVIEW_TIMEOUT: Duration = Duration::from_secs(600);

pub fn run(
    stream: &mut UnixStream,
    args: Vec<String>,
    cwd: PathBuf,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> Result<(), String> {
    if args.is_empty() {
        return Err("cargo: no subcommand".to_string());
    }
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    if let Err(e) = zap_wire::cargo_gate::check(&refs) {
        return Err(e);
    }

    let title = format!("cargo {}", args.join(" "));

    let Some(root) = surface::project_root(&cwd) else {
        let _ = write_frame(stream, &Frame::stream_open(&title));
        let _ = write_frame(
            stream,
            &Frame::stream_data("cargo: no Cargo.toml at or above this folder\n"),
        );
        let _ = write_frame(stream, &Frame::stream_done("cargo: no project"));
        return Ok(());
    };

    let surface_now = match surface::collect(&root) {
        Ok(s) => s,
        Err(e) => {
            let _ = write_frame(stream, &Frame::stream_open(&title));
            let _ = write_frame(stream, &Frame::stream_data(&format!("{e}\n")));
            let _ = write_frame(stream, &Frame::stream_done("surface error"));
            return Ok(());
        }
    };

    let diffs: Vec<SurfaceDiff> = match trust::load(&root) {
        Some(record) => trust::compare(&record, &surface_now),
        None => surface_now
            .iter()
            .map(|f| SurfaceDiff {
                path: f.path.clone(),
                old_text: None,
                new_text: f.text.clone(),
            })
            .collect(),
    };

    if !diffs.is_empty() {
        let (tx, rx) = std::sync::mpsc::channel();
        let caption = format!("execution surface changed for {}", root.display());
        let review = crate::terminal_router::surface_review::SurfaceReview::new(
            root.clone(),
            diffs,
            surface_now.clone(),
            tx,
            crate::terminal_router::surface_review::BannerKind::CargoRun,
            caption,
        );
        if !crate::terminal_router::stash_surface_review(review) {
            let _ = write_frame(stream, &Frame::stream_open(&title));
            let _ = write_frame(
                stream,
                &Frame::stream_data("cargo: another surface review is showing\n"),
            );
            let _ = write_frame(stream, &Frame::stream_done("busy"));
            return Ok(());
        }
        let accepted = rx.recv_timeout(SURFACE_REVIEW_TIMEOUT).unwrap_or(false);
        if !accepted {
            let _ = write_frame(stream, &Frame::stream_open(&title));
            let _ = write_frame(stream, &Frame::stream_data("cargo: surface review denied\n"));
            let _ = write_frame(stream, &Frame::stream_done("denied"));
            crate::chain::popup::show("Run denied");
            let _ = clipboard::write(
                &format!("zap: cargo run denied for {}", root.display()),
                RecordKind::Report,
                WriteMode::Replace,
            );
            return Ok(());
        }
        let fresh = surface::collect(&root)?;
        if fresh != surface_now {
            let _ = write_frame(stream, &Frame::stream_open(&title));
            let _ = write_frame(
                stream,
                &Frame::stream_data("cargo: surface changed during review\n"),
            );
            let _ = write_frame(stream, &Frame::stream_done("changed"));
            return Ok(());
        }
        let record = trust::from_surface(&root, &fresh);
        trust::save(&record)?;
    }

    let root_for_after = root.clone();
    let spec = StreamSpec {
        run: RunSpec {
            program: "cargo".to_string(),
            args: args.clone(),
            cwd: cwd.clone(),
            env: runner::pane_env(),
        },
        title,
        summary_prefix: format!("$ cargo {} (in {}, ", args.join(" "), cwd.display()),
        popup_ok: "Cargo run: ok".to_string(),
        popup_fail: "Cargo run failed".to_string(),
        after: Some(Box::new(move |_output: &str| {
            if let Some(mut record) = trust::load(&root_for_after) {
                if let Ok(fresh) = surface::collect(&root_for_after) {
                    trust::refresh_lock(&mut record, &fresh);
                    let _ = trust::save(&record);
                }
            }
        })),
        cancel_flag,
    };
    crate::terminal_router::stream_run::stream(stream, spec)
}