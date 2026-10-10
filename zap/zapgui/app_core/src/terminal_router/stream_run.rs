//! The shared process-streaming loop. Both the cargo verb and the run verb
//! go through here. Spawns the process, forwards its output as stream
//! frames, watches for cancel or EOF, then writes a summary and a popup.

use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use zap_wire::{read_frame, write_frame, Frame, FrameKind};

use crate::clipboard::{self, RecordKind, WriteMode};
use crate::runner::{self, RunEvent, RunSpec};

const SUMMARY_CAP: usize = 20 * 1024;

pub struct StreamSpec {
    pub run: RunSpec,
    pub title: String,
    /// Prefix of the summary header. The status is appended as
    /// "<prefix><status>)".
    pub summary_prefix: String,
    pub popup_ok: String,
    pub popup_fail: String,
    /// Called once after the process ends, with the raw output. Used by
    /// cargo to refresh Cargo.lock.
    pub after: Option<Box<dyn FnOnce(&str) + Send>>,
}

pub fn stream(stream: &mut UnixStream, spec: StreamSpec) -> Result<(), String> {
    let _ = write_frame(stream, &Frame::stream_open(&spec.title));

    let run = match runner::spawn(spec.run) {
        Ok(r) => r,
        Err(e) => {
            let _ = write_frame(stream, &Frame::stream_data(&format!("{e}\n")));
            let _ = write_frame(stream, &Frame::stream_done("spawn failed"));
            return Ok(());
        }
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut reader) = stream.try_clone() {
        let cf = cancel_flag.clone();
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

    let mut output = String::new();
    let mut cancelled = false;
    loop {
        if cancel_flag.load(Ordering::SeqCst) && !cancelled {
            run.cancel();
            cancelled = true;
        }
        match run.recv_timeout(Duration::from_millis(80)) {
            Ok(RunEvent::Data(t)) => {
                output.push_str(&t);
                let _ = write_frame(stream, &Frame::stream_data(&t));
            }
            Ok(RunEvent::Done(status)) => {
                let _ = write_frame(stream, &Frame::stream_done(&status));
                write_summary(&spec.summary_prefix, &status, &output);
                if let Some(after) = spec.after {
                    after(&output);
                }
                let short = if cancelled {
                    "Run cancelled"
                } else if status == "exit 0" {
                    spec.popup_ok.as_str()
                } else {
                    spec.popup_fail.as_str()
                };
                crate::chain::popup::show(short);
                return Ok(());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                let _ = write_frame(stream, &Frame::stream_done("unknown"));
                return Ok(());
            }
        }
    }
}

fn write_summary(prefix: &str, status: &str, output: &str) {
    let header = format!("{prefix}{status})");
    let mut folder = zap_wire::linefold::LineFolder::default();
    folder.push_str(output);
    folder.finish();
    let folded = folder.lines.join("\n");
    let cleaned = strip_noise(&folded);
    let capped = cap_output(&cleaned, SUMMARY_CAP);
    let summary = format!("{header}\n{capped}");
    let _ = clipboard::write(&summary, RecordKind::RunOutput, WriteMode::Append);
}

fn strip_noise(s: &str) -> String {
    let mut out = String::new();
    for line in s.lines() {
        let trimmed = line.trim_start();
        let noise = trimmed.starts_with("Compiling ")
            || trimmed.starts_with("Downloaded ")
            || trimmed.starts_with("Updating ")
            || trimmed.starts_with("Blocking ")
            || trimmed.starts_with("Building [")
            || trimmed.starts_with("   Compiling ")
            || trimmed.starts_with("   Downloaded ")
            || trimmed.starts_with("   Updating ")
            || trimmed == "Finished `dev` profile [unoptimized + debuginfo] target(s)"
            || trimmed.starts_with("Finished `dev` profile [unoptimized + debuginfo] target(s) in");
        if noise {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn cap_output(s: &str, cap: usize) -> String {
    if s.len() <= cap {
        return s.to_string();
    }
    let keep = cap / 2;
    let head: String = s.chars().take(keep).collect();
    let tail: String = s
        .chars()
        .rev()
        .take(keep)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let dropped = s.len().saturating_sub(head.len() + tail.len());
    format!("{head}\n... [{dropped} bytes cut] ...\n{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_noise_drops_compiling_lines() {
        let input = "Compiling a v1\nwarning: unused\n   Compiling b v2\nerror: boom\n";
        let out = strip_noise(input);
        assert!(!out.contains("Compiling"));
        assert!(out.contains("warning: unused"));
        assert!(out.contains("error: boom"));
    }

    #[test]
    fn cap_keeps_head_and_tail() {
        let long = "a".repeat(1000);
        let capped = cap_output(&long, 100);
        assert!(capped.len() < long.len());
        assert!(capped.contains("bytes cut"));
    }

    #[test]
    fn cap_is_noop_when_short() {
        assert_eq!(cap_output("hello", 100), "hello");
    }
}
