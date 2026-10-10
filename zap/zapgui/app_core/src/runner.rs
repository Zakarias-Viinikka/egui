//! A generic process runner. Spawns a program in its own process group,
//! merges stdout and stderr in arrival order, strips ANSI escapes, caps the
//! captured output, and kills the whole group on cancel or drop. Nothing
//! in here knows about cargo; the cargo verb passes the program and env.

use std::io::Read;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Stop reading once we've got this much. The plan says a couple of MB.
const OUTPUT_CAP_BYTES: usize = 2 * 1024 * 1024;
/// Wait this long after SIGTERM before SIGKILL.
const KILL_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub enum RunEvent {
    Data(String),
    Done(String),
}

pub struct RunSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// Extra environment variables, on top of the parent's. Cargo callers
    /// pass the colour and progress suppressions here.
    pub env: Vec<(String, String)>,
}

pub struct Run {
    cancel_flag: Arc<AtomicBool>,
    events: Receiver<RunEvent>,
}

impl Run {
    /// Ask the supervisor thread to terminate the process group. Safe to
    /// call from anywhere; does nothing if the run already finished.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn try_recv(&self) -> Option<RunEvent> {
        self.events.try_recv().ok()
    }

    pub fn recv(&self) -> Option<RunEvent> {
        self.events.recv().ok()
    }

    pub fn recv_timeout(&self, d: Duration) -> Result<RunEvent, std::sync::mpsc::RecvTimeoutError> {
        self.events.recv_timeout(d)
    }
}

pub fn spawn(spec: RunSpec) -> Result<Run, String> {
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    // 0 means "new process group, pgid equals the child's pid".
    cmd.process_group(0);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn {}: {}", spec.program, e))?;
    let pgid = child.id() as i32;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let (tx, rx) = channel();
    let tx_out = tx.clone();
    let tx_err = tx.clone();
    let tx_done = tx.clone();
    drop(tx);

    let h_out = thread::spawn(move || read_stream(stdout, tx_out, true));
    let h_err = thread::spawn(move || read_stream(stderr, tx_err, false));

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_for_thread = cancel_flag.clone();

    thread::spawn(move || supervisor(child, pgid, cancel_for_thread, tx_done, h_out, h_err));

    Ok(Run { cancel_flag, events: rx })
}

fn read_stream<R: Read + Send + 'static>(mut r: R, tx: std::sync::mpsc::Sender<RunEvent>, capped: bool) {
    let mut buf = [0u8; 4096];
    let mut total = 0usize;
    // Bytes left over from a previous read that look like the start of a
    // multi-byte UTF-8 sequence. Prepended to the next read.
    let mut carry: Vec<u8> = Vec::new();
    loop {
        match r.read(&mut buf) {
            Ok(0) => {
                if !carry.is_empty() {
                    let lossy = String::from_utf8_lossy(&carry).into_owned();
                    let cleaned = strip_ansi(&lossy);
                    if !cleaned.is_empty() {
                        let _ = tx.send(RunEvent::Data(cleaned));
                    }
                }
                break;
            }
            Ok(n) => {
                if capped && total >= OUTPUT_CAP_BYTES {
                    continue;
                }
                let mut chunk = std::mem::take(&mut carry);
                chunk.extend_from_slice(&buf[..n]);
                let valid_up_to = match std::str::from_utf8(&chunk) {
                    Ok(_) => chunk.len(),
                    Err(e) => e.valid_up_to(),
                };
                let tail = chunk.len() - valid_up_to;
                if tail > 0 && tail < 4 {
                    // Partial multi-byte sequence at the end; hold it.
                    carry = chunk[valid_up_to..].to_vec();
                    chunk.truncate(valid_up_to);
                } else {
                    carry.clear();
                }
                if chunk.is_empty() {
                    continue;
                }
                let take = if capped {
                    (OUTPUT_CAP_BYTES - total).min(chunk.len())
                } else {
                    chunk.len()
                };
                total += take;
                let lossy = String::from_utf8_lossy(&chunk[..take]).into_owned();
                let cleaned = strip_ansi(&lossy);
                if !cleaned.is_empty() && tx.send(RunEvent::Data(cleaned)).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// The environment a program run through the pane should have: no colour,
/// progress bar on, width pinned, TERM set so tools that key off it draw
/// their bar. Shared by the cargo verb and the run verb.
pub fn pane_env() -> Vec<(String, String)> {
    vec![
        ("CARGO_TERM_COLOR".into(), "never".into()),
        ("CARGO_TERM_PROGRESS_WHEN".into(), "always".into()),
        ("CARGO_TERM_PROGRESS_WIDTH".into(), "60".into()),
        ("NO_COLOR".into(), "1".into()),
        ("TERM".into(), "xterm-256color".into()),
    ]
}

fn supervisor(
    mut child: Child,
    pgid: i32,
    cancel_flag: Arc<AtomicBool>,
    tx_done: std::sync::mpsc::Sender<RunEvent>,
    h_out: thread::JoinHandle<()>,
    h_err: thread::JoinHandle<()>,
) {
    let mut cancelled_at: Option<Instant> = None;
    let mut killed = false;
    loop {
        if cancel_flag.load(Ordering::SeqCst) && cancelled_at.is_none() {
            unsafe {
                libc::kill(-pgid, libc::SIGTERM);
            }
            cancelled_at = Some(Instant::now());
        }
        if let Some(t) = cancelled_at {
            if !killed && t.elapsed() >= KILL_GRACE {
                unsafe {
                    libc::kill(-pgid, libc::SIGKILL);
                }
                killed = true;
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                // Wait for both readers to drain before Done, so all Data
                // frames are queued ahead of the terminal event.
                let _ = h_out.join();
                let _ = h_err.join();
                let _ = tx_done.send(RunEvent::Done(format_exit(status)));
                break;
            }
            Ok(None) => {}
            Err(e) => {
                let _ = tx_done.send(RunEvent::Done(format!("wait failed: {e}")));
                break;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn format_exit(status: std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("exit {code}");
    }
    if let Some(sig) = status.signal() {
        return format!("signal {sig}");
    }
    "unknown".to_string()
}

/// Drop ANSI escape sequences: CSI (ESC [ ... final 0x40-0x7E) and simple
/// two-character escapes. Everything else passes through unchanged.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if ('\x40'..='\x7e').contains(&nc) {
                        break;
                    }
                }
            }
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_colour_csi() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi("\x1b[1;32mgreen\x1b[0m plain"), "green plain");
    }

    #[test]
    fn leaves_plain_text_alone() {
        assert_eq!(strip_ansi("no escapes here"), "no escapes here");
        assert_eq!(strip_ansi("trailing \x1b"), "trailing ");
    }

    #[test]
    fn strips_two_char_escapes() {
        assert_eq!(strip_ansi("a\x1bMb"), "ab");
    }

    #[test]
    fn captures_hello_output() {
        let run = spawn(RunSpec {
            program: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            cwd: PathBuf::from("/"),
            env: Vec::new(),
        })
        .expect("spawn");
        let mut output = String::new();
        let mut done = None;
        while let Some(ev) = run.recv() {
            match ev {
                RunEvent::Data(t) => output.push_str(&t),
                RunEvent::Done(s) => {
                    done = Some(s);
                    break;
                }
            }
        }
        assert!(output.contains("hello"));
        assert_eq!(done.as_deref(), Some("exit 0"));
    }

    #[test]
    fn reports_nonzero_exit() {
        let run = spawn(RunSpec {
            program: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), "exit 3".to_string()],
            cwd: PathBuf::from("/"),
            env: Vec::new(),
        })
        .expect("spawn");
        let mut done = None;
        while let Some(ev) = run.recv() {
            if let RunEvent::Done(s) = ev {
                done = Some(s);
                break;
            }
        }
        assert_eq!(done.as_deref(), Some("exit 3"));
    }

    #[test]
    fn cancel_kills_sleep() {
        let run = spawn(RunSpec {
            program: "/bin/sleep".to_string(),
            args: vec!["10".to_string()],
            cwd: PathBuf::from("/"),
            env: Vec::new(),
        })
        .expect("spawn");
        thread::sleep(Duration::from_millis(200));
        run.cancel();
        let mut done = None;
        while let Some(ev) = run.recv() {
            if let RunEvent::Done(s) = ev {
                done = Some(s);
                break;
            }
        }
        let s = done.expect("done event");
        assert!(s.starts_with("signal "), "got {s}");
    }
}
