//! Clipboard access via xclip. Write with a mode, read with a timeout, and
//! keep a small in-memory record of the last text zap wrote so run output
//! can be appended under a chain report without stomping whatever the user
//! copied themselves.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long xclip is allowed to run before we give up. A short timeout
/// because a missing or hung clipboard must not block the UI.
const TIMEOUT: Duration = Duration::from_millis(800);
/// Append mode only fires if the last write was a report and it's younger
/// than this. Older than that and the user has moved on.
const APPEND_WINDOW: Duration = Duration::from_secs(600);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteMode {
    /// Overwrite the clipboard with the given text. Used by chain reports
    /// and by run decisions, which already hold the full picture.
    Replace,
    /// If the clipboard still holds what zap last wrote, and that was a
    /// report, and it's recent, append the new text below it. Otherwise
    /// replace. Used by run output.
    Append,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    Report,
    RunOutput,
}

struct Record {
    text: String,
    at: Instant,
    kind: RecordKind,
}

static LAST_WRITE: Mutex<Option<Record>> = Mutex::new(None);

/// Write `text` to the clipboard under the given mode. The record is
/// updated either way. Returns Err with a human-readable reason if xclip
/// fails, so a failed write can't look like success.
pub fn write(text: &str, kind: RecordKind, mode: WriteMode) -> Result<(), String> {
    let final_text = match mode {
        WriteMode::Replace => text.to_string(),
        WriteMode::Append => append_or_replace(text),
    };

    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("xclip could not start: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(final_text.as_bytes())
            .map_err(|e| format!("xclip write failed: {e}"))?;
    }

    let status = child.wait().map_err(|e| format!("xclip wait failed: {e}"))?;
    if !status.success() {
        return Err(format!("xclip exited with {status}"));
    }

    if let Ok(mut g) = LAST_WRITE.lock() {
        *g = Some(Record { text: final_text, at: Instant::now(), kind });
    }
    Ok(())
}

/// Back-compat shim for callers that want today's behaviour: replace, and
/// record it as a report.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    write(text, RecordKind::Report, WriteMode::Replace)
}

/// Read the clipboard, or None on any failure or empty result. "Unknown"
/// is the right answer when reading fails: append mode then replaces.
pub fn read() -> Option<String> {
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // xclip -o blocks until another client owns the selection, so this
    // has to be bounded. Read on a worker thread and wait with a timeout.
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });

    let buf = match rx.recv_timeout(TIMEOUT) {
        Ok(b) => b,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };
    let _ = child.wait();

    if buf.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn append_or_replace(new_text: &str) -> String {
    let Ok(g) = LAST_WRITE.lock() else {
        return new_text.to_string();
    };
    let Some(record) = g.as_ref() else {
        return new_text.to_string();
    };
    if record.kind != RecordKind::Report {
        return new_text.to_string();
    }
    if record.at.elapsed() > APPEND_WINDOW {
        return new_text.to_string();
    }
    let Some(current) = read() else {
        return new_text.to_string();
    };
    if current.trim_end() != record.text.trim_end() {
        return new_text.to_string();
    }
    let mut out = record.text.clone();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(new_text);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The append decision is pure enough to test without a real clipboard
    /// by pushing a record and calling append_or_replace with a stub read
    /// replacement. To keep the test hermetic we just check the branches
    /// that don't need read() — no record, wrong kind, stale — all fall
    /// back to replace.
    #[test]
    fn no_record_means_replace() {
        let g = LAST_WRITE.lock().unwrap();
        drop(g);
        // Reset the record for this test process.
        *LAST_WRITE.lock().unwrap() = None;
        assert_eq!(append_or_replace("new"), "new");
    }

    #[test]
    fn run_output_record_means_replace() {
        *LAST_WRITE.lock().unwrap() = Some(Record {
            text: "old run output".to_string(),
            at: Instant::now(),
            kind: RecordKind::RunOutput,
        });
        assert_eq!(append_or_replace("new"), "new");
        *LAST_WRITE.lock().unwrap() = None;
    }

    #[test]
    fn stale_report_means_replace() {
        *LAST_WRITE.lock().unwrap() = Some(Record {
            text: "old report".to_string(),
            at: Instant::now() - Duration::from_secs(3600),
            kind: RecordKind::Report,
        });
        assert_eq!(append_or_replace("new"), "new");
        *LAST_WRITE.lock().unwrap() = None;
    }
}
