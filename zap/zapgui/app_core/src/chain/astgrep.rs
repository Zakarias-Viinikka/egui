//! The engine behind `astgrep rename`: find the installed ast-grep binary,
//! run it read-only as a subprocess, and turn its matches into per-file
//! before and after texts. Nothing here writes to the project.
//!
//! The binary is probed by capability, never by name alone: `--version`
//! must start with `ast-grep` and `scan --help` must mention
//! `--inline-rules` and `--json`. `sg` is never called.

use std::collections::BTreeMap;
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use serde::Deserialize;
use zap_wire::hidden::HiddenSet;

#[cfg(test)]
use std::cell::RefCell;

/// A rename touching more files than this fails.
pub const MAX_FILES: usize = 200;
/// A rename whose changed files add up to more text than this fails.
pub const MAX_CHANGED_BYTES: usize = 4 * 1024 * 1024;

const MAX_STDOUT_BYTES: usize = 64 * 1024 * 1024;
const MAX_STDERR_BYTES: usize = 64 * 1024;
const SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

const NOT_FOUND: &str = "ast-grep was not found in ZAP_AST_GREP, PATH, ~/.cargo/bin, ~/.local/bin, /usr/local/bin or /usr/bin";
const TOO_OLD: &str = "ast-grep is too old, it needs --inline-rules and --json";

/// One file the rename changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEdit {
    pub path: PathBuf,
    pub before: String,
    pub after: String,
    pub occurrences: usize,
}

/// One match as ast-grep prints it. Only the file and the byte offsets
/// are read; every other field is ignored.
#[derive(Debug, Deserialize)]
pub struct RawMatch {
    pub file: String,
    pub range: RawRange,
}

#[derive(Debug, Deserialize)]
pub struct RawRange {
    #[serde(rename = "byteOffset")]
    pub byte_offset: RawOffsets,
}

#[derive(Debug, Deserialize)]
pub struct RawOffsets {
    pub start: usize,
    pub end: usize,
}

// ---- finding the binary ----

/// Successes only, so installing ast-grep while zapgui runs needs no
/// restart.
static FOUND: OnceLock<PathBuf> = OnceLock::new();

#[cfg(test)]
thread_local! {
    static OVERRIDE: RefCell<Option<PathBuf>> = RefCell::new(None);
}

/// Tests point the finder at a fake script. Per thread, so tests that
/// run side by side do not see each other's.
#[cfg(test)]
pub(crate) fn set_override(path: Option<PathBuf>) {
    OVERRIDE.with(|o| *o.borrow_mut() = path);
}

enum Probe {
    Usable,
    TooOld,
    Missing,
}

fn candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("ZAP_AST_GREP") {
        if !p.is_empty() {
            out.push(PathBuf::from(p));
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            out.push(dir.join("ast-grep"));
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        out.push(home.join(".cargo").join("bin").join("ast-grep"));
        out.push(home.join(".local").join("bin").join("ast-grep"));
    }
    out.push(PathBuf::from("/usr/local/bin/ast-grep"));
    out.push(PathBuf::from("/usr/bin/ast-grep"));
    out
}

fn probe(bin: &Path) -> Probe {
    if !bin.is_file() {
        return Probe::Missing;
    }
    let mut version = Command::new(bin);
    version.arg("--version");
    // A candidate that cannot even print its version is not a usable
    // ast-grep, so the run error becomes Missing here instead of a report.
    let Ok(cap) = run_capture(&mut version, PROBE_TIMEOUT, 64 * 1024) else {
        return Probe::Missing;
    };
    let said = String::from_utf8_lossy(&cap.stdout).into_owned();
    if cap.code != Some(0) || !said.trim_start().starts_with("ast-grep") {
        return Probe::Missing;
    }
    let mut help = Command::new(bin);
    help.args(["scan", "--help"]);
    match run_capture(&mut help, PROBE_TIMEOUT, 1024 * 1024) {
        Ok(c) => {
            let text = String::from_utf8_lossy(&c.stdout);
            if text.contains("--inline-rules") && text.contains("--json") {
                Probe::Usable
            } else {
                Probe::TooOld
            }
        }
        // A binary whose help cannot be read cannot be shown to have the
        // flags, which is what "too old" says.
        Err(_) => Probe::TooOld,
    }
}

/// The usable ast-grep binary, or the one-clause reason there is none.
pub fn find_binary() -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        let forced = OVERRIDE.with(|o| o.borrow().clone());
        if let Some(p) = forced {
            return match probe(&p) {
                Probe::Usable => Ok(p),
                Probe::TooOld => Err(TOO_OLD.to_string()),
                Probe::Missing => Err(NOT_FOUND.to_string()),
            };
        }
    }
    if let Some(p) = FOUND.get() {
        return Ok(p.clone());
    }
    let mut too_old = false;
    for cand in candidates() {
        match probe(&cand) {
            Probe::Usable => {
                // set fails only when another thread cached first, and
                // then the stored path is also a usable one.
                let _ = FOUND.set(cand.clone());
                return Ok(cand);
            }
            Probe::TooOld => too_old = true,
            Probe::Missing => {}
        }
    }
    Err(if too_old { TOO_OLD } else { NOT_FOUND }.to_string())
}

// ---- running it ----

struct Captured {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

enum RunError {
    Spawn(String),
    Timeout,
    TooBig,
    Io(String),
}

fn stop(child: &mut Child) -> Result<(), String> {
    match child.kill() {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {}
        Err(e) => return Err(format!("kill: {e}")),
    }
    child.wait().map(|_| ()).map_err(|e| format!("wait: {e}"))
}

/// Read a pipe to its end. Past `cap` bytes: with `over` set, raise the
/// flag and stop; without it, keep draining and keep nothing more.
fn read_capped<R: Read>(
    mut r: R,
    cap: usize,
    over: Option<Arc<AtomicBool>>,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let n = match r.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("read: {e}")),
        };
        let room = cap.saturating_sub(buf.len());
        if n > room {
            buf.extend_from_slice(&chunk[..room]);
            if let Some(flag) = &over {
                flag.store(true, Ordering::SeqCst);
                return Ok(buf);
            }
        } else {
            buf.extend_from_slice(&chunk[..n]);
        }
    }
    Ok(buf)
}

fn join_reader(
    h: std::thread::JoinHandle<Result<Vec<u8>, String>>,
    which: &str,
) -> Result<Vec<u8>, RunError> {
    match h.join() {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(RunError::Io(format!("{which}: {e}"))),
        Err(_) => Err(RunError::Io(format!("the {which} reader stopped unexpectedly"))),
    }
}

/// Run a command with stdout and stderr drained on their own threads and
/// the exit polled, so a full pipe cannot hang it. Killed on timeout, and
/// when stdout passes `max_stdout`.
fn run_capture(
    cmd: &mut Command,
    timeout: Duration,
    max_stdout: usize,
) -> Result<Captured, RunError> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| RunError::Spawn(e.to_string()))?;
    let (Some(out_pipe), Some(err_pipe)) = (child.stdout.take(), child.stderr.take()) else {
        stop(&mut child).map_err(RunError::Io)?;
        return Err(RunError::Io("the pipes were not set up".to_string()));
    };
    let over = Arc::new(AtomicBool::new(false));
    let over_for_reader = over.clone();
    let t_out = std::thread::spawn(move || read_capped(out_pipe, max_stdout, Some(over_for_reader)));
    let t_err = std::thread::spawn(move || read_capped(err_pipe, MAX_STDERR_BYTES, None));

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) => {}
            Err(e) => {
                stop(&mut child).map_err(RunError::Io)?;
                return Err(RunError::Io(format!("wait: {e}")));
            }
        }
        if over.load(Ordering::SeqCst) {
            stop(&mut child).map_err(RunError::Io)?;
            break None;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            stop(&mut child).map_err(RunError::Io)?;
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = join_reader(t_out, "stdout")?;
    let stderr = join_reader(t_err, "stderr")?;
    if timed_out {
        return Err(RunError::Timeout);
    }
    if over.load(Ordering::SeqCst) {
        return Err(RunError::TooBig);
    }
    Ok(Captured {
        code: status.and_then(|s| s.code()),
        stdout,
        stderr,
    })
}

fn run_error_text(e: RunError, timeout: Duration) -> String {
    match e {
        RunError::Spawn(m) => format!("couldn't start ast-grep: {m}"),
        RunError::Timeout => format!("ast-grep took longer than {} s", timeout.as_secs()),
        RunError::TooBig => format!(
            "ast-grep printed more than {} MB",
            MAX_STDOUT_BYTES / (1024 * 1024)
        ),
        RunError::Io(m) => format!("ast-grep run failed: {m}"),
    }
}

/// The first 200 characters of stderr on one line.
fn short_stderr(stderr: &[u8]) -> String {
    let text: String = String::from_utf8_lossy(stderr)
        .chars()
        .take(200)
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let t = text.trim();
    if t.is_empty() {
        "no message".to_string()
    } else {
        t.to_string()
    }
}

/// ~/.cache/zap/astgrep-cwd, mode 0700. ast-grep runs with this as its
/// working folder so it never picks up a project's sgconfig.yml.
fn private_cwd() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    let dir = PathBuf::from(home)
        .join(".cache")
        .join("zap")
        .join("astgrep-cwd");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)
        .map_err(|e| format!("couldn't make {}: {e}", dir.display()))?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("couldn't set the mode of {}: {e}", dir.display()))?;
    Ok(dir)
}

/// The inline rule. `old` has been checked as a plain identifier, so it
/// needs no escaping. A lifetime or a loop label is an identifier node
/// inside a `lifetime` or `label` node, so those are excluded.
fn rule_text(old: &str) -> String {
    format!(
        "id: zap-rename
language: rust
severity: hint
message: zap rename
rule:
  all:
    - regex: '^{old}$'
    - any:
        - kind: identifier
        - kind: type_identifier
        - kind: field_identifier
        - kind: shorthand_field_identifier
    - not:
        inside:
          any:
            - kind: lifetime
            - kind: label
"
    )
}

/// Accepts a stream (one object per line) or an array.
pub fn parse_matches(stdout: &[u8]) -> Result<Vec<RawMatch>, String> {
    let text = std::str::from_utf8(stdout)
        .map_err(|e| format!("ast-grep printed text that is not UTF-8: {e}"))?;
    let t = text.trim();
    if t.is_empty() {
        return Ok(Vec::new());
    }
    if t.starts_with('[') {
        return serde_json::from_str::<Vec<RawMatch>>(t)
            .map_err(|e| format!("ast-grep printed JSON that did not parse: {e}"));
    }
    let mut out: Vec<RawMatch> = Vec::new();
    for line in t.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let m: RawMatch = serde_json::from_str(l)
            .map_err(|e| format!("ast-grep printed a line that did not parse: {e}"))?;
        out.push(m);
    }
    Ok(out)
}

fn scan_with(
    bin: &Path,
    folder: &Path,
    old: &str,
    timeout: Duration,
) -> Result<Vec<RawMatch>, String> {
    let cwd = private_cwd()?;
    let mut cmd = Command::new(bin);
    cmd.current_dir(&cwd)
        .args(["scan", "--color", "never", "--json=stream", "--inline-rules"])
        .arg(rule_text(old))
        .arg(folder);
    let cap = run_capture(&mut cmd, timeout, MAX_STDOUT_BYTES)
        .map_err(|e| run_error_text(e, timeout))?;
    match cap.code {
        Some(0) => {}
        Some(c) => {
            return Err(format!(
                "ast-grep exited with status {c}: {}",
                short_stderr(&cap.stderr)
            ));
        }
        None => return Err("ast-grep was stopped by a signal".to_string()),
    }
    parse_matches(&cap.stdout)
}

// ---- from matches to edits ----

/// A path under `target`, under `.git`, or under any dot-directory, is
/// never renamed. `rel` is relative to the folder.
fn skipped_component(rel: &Path) -> bool {
    let comps: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let last = comps.len().saturating_sub(1);
    comps.iter().enumerate().any(|(i, name)| {
        name == "target" || name == ".git" || (i < last && name.starts_with('.'))
    })
}

/// Turn matches into per-file edits, sorted by path. `read` returns a
/// file's current text; the caller makes it record a snapshot.
pub fn edits_from(
    matches: &[RawMatch],
    folder: &Path,
    old: &str,
    new: &str,
    hidden: &HiddenSet,
    read: &mut dyn FnMut(&Path) -> Result<Option<String>, String>,
) -> Result<Vec<FileEdit>, String> {
    let mut by_file: BTreeMap<PathBuf, Vec<(usize, usize)>> = BTreeMap::new();
    for m in matches {
        let path = PathBuf::from(&m.file);
        let rel = path.strip_prefix(folder).map_err(|_| {
            format!(
                "ast-grep returned {}, which is outside {}",
                path.display(),
                folder.display()
            )
        })?;
        if skipped_component(rel) {
            continue;
        }
        by_file
            .entry(path)
            .or_default()
            .push((m.range.byte_offset.start, m.range.byte_offset.end));
    }
    if by_file.is_empty() {
        return Err(format!(
            "no identifier named `{old}` was found under {}",
            folder.display()
        ));
    }
    if by_file.len() > MAX_FILES {
        return Err(format!(
            "the rename touches {} files, the limit is {MAX_FILES}",
            by_file.len()
        ));
    }

    let mut out: Vec<FileEdit> = Vec::new();
    let mut total = 0usize;
    for (path, mut ranges) in by_file {
        if let Some(msg) = hidden.refusal(&path) {
            return Err(msg);
        }
        let before = read(&path)?
            .ok_or_else(|| format!("{} does not exist", path.display()))?;
        ranges.sort_unstable();
        for w in ranges.windows(2) {
            if w[1].0 < w[0].1 {
                return Err(format!(
                    "{}: ast-grep gave overlapping matches",
                    path.display()
                ));
            }
        }
        let mut after = String::with_capacity(before.len());
        let mut at = 0usize;
        for &(s, e) in &ranges {
            if before.get(s..e) != Some(old) {
                return Err(format!(
                    "{}: the text at bytes {s}..{e} is not `{old}`, so the file changed since ast-grep read it",
                    path.display()
                ));
            }
            let Some(gap) = before.get(at..s) else {
                return Err(format!("{}: ast-grep gave matches out of order", path.display()));
            };
            after.push_str(gap);
            after.push_str(new);
            at = e;
        }
        let Some(tail) = before.get(at..) else {
            return Err(format!("{}: ast-grep gave a match past the end", path.display()));
        };
        after.push_str(tail);
        total += after.len();
        if total > MAX_CHANGED_BYTES {
            return Err(format!(
                "the rename changes more than {} MB of text",
                MAX_CHANGED_BYTES / (1024 * 1024)
            ));
        }
        out.push(FileEdit {
            path,
            before,
            after,
            occurrences: ranges.len(),
        });
    }
    Ok(out)
}

/// Run ast-grep over `folder` and plan the rename.
pub fn plan(
    folder: &Path,
    old: &str,
    new: &str,
    hidden: &HiddenSet,
    read: &mut dyn FnMut(&Path) -> Result<Option<String>, String>,
) -> Result<Vec<FileEdit>, String> {
    let bin = find_binary()?;
    let matches = scan_with(&bin, folder, old, SCAN_TIMEOUT)?;
    edits_from(&matches, folder, old, new, hidden, read)
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// A shell script that stands in for ast-grep, and the override that
    /// points the finder at it. `scan_body` runs for a real scan.
    pub fn fake_ast_grep(
        dir: &Path,
        version_line: &str,
        help_text: &str,
        scan_body: &str,
    ) -> PathBuf {
        let path = dir.join("fake-ast-grep");
        let script = format!(
            "#!/bin/sh\ncase \"$1\" in\n  --version) echo '{version_line}' ;;\n  scan)\n    if [ \"$2\" = \"--help\" ]; then echo '{help_text}'; exit 0; fi\n    {scan_body}\n    ;;\nesac\n"
        );
        std::fs::write(&path, script).expect("write fake ast-grep");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake ast-grep");
        set_override(Some(path.clone()));
        path
    }

    /// A fake that passes the probe and prints `stdout` for every scan.
    pub fn healthy(dir: &Path, stdout: &str) -> PathBuf {
        std::fs::write(dir.join("out.jsonl"), stdout).expect("write fake output");
        fake_ast_grep(
            dir,
            "ast-grep 9.9.9",
            "--inline-rules <RULE_TEXT> --json[=<STYLE>]",
            "cat \"$(dirname \"$0\")/out.jsonl\"",
        )
    }

    /// One stream line in the shape the real binary prints.
    pub fn match_line(file: &Path, start: usize, end: usize) -> String {
        format!(
            "{{\"text\":\"x\",\"range\":{{\"byteOffset\":{{\"start\":{start},\"end\":{end}}}}},\"file\":\"{}\"}}\n",
            file.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{fake_ast_grep, healthy, match_line};
    use super::*;
    use std::fs;

    const PROBE1_FOO: &str = include_str!("../../fixtures/astgrep/probe1_foo.jsonl");
    const PROBE1_A: &str = include_str!("../../fixtures/astgrep/probe1_a.jsonl");
    const PROBE2_A: &str = include_str!("../../fixtures/astgrep/probe2_a.jsonl");
    const PROBE2_ZZZ: &str = include_str!("../../fixtures/astgrep/probe2_zzz.jsonl");

    /// What the probe wrote to src/lib.rs.
    const LIB_RS: &str = "fn l<'a>(x: &'a str) -> &'a str { x }\nfn m() { 'a: loop { break 'a; } let a = 1; }\n";

    fn tmp() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("zap_astgrep_test_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    /// The probe's folder, rebuilt under `dir`.
    fn project(dir: &Path) {
        for sub in ["src/sub", "target", "ign"] {
            fs::create_dir_all(dir.join(sub)).expect("mkdir");
        }
        fs::write(dir.join("src/lib.rs"), LIB_RS).expect("write");
        fs::write(dir.join("src/sub/inner.rs"), "fn a() {}\n").expect("write");
        fs::write(dir.join("target/t.rs"), "fn a() {}\n").expect("write");
        fs::write(dir.join("ign/i.rs"), "fn a() {}\n").expect("write");
    }

    fn fixture_for(dir: &Path) -> String {
        PROBE2_A.replace("/tmp/zap_astgrep_probe2", &dir.display().to_string())
    }

    fn reader() -> impl FnMut(&Path) -> Result<Option<String>, String> {
        |p: &Path| crate::chain::play::read_or_none(p)
    }

    #[test]
    fn real_stream_output_parses() {
        let m = parse_matches(PROBE1_FOO.as_bytes()).expect("parse");
        assert_eq!(m.len(), 10);
        assert_eq!(m[0].file, "/tmp/zap_astgrep_probe/probe.rs");
        assert_eq!((m[0].range.byte_offset.start, m[0].range.byte_offset.end), (3, 6));
        let a = parse_matches(PROBE1_A.as_bytes()).expect("parse");
        assert_eq!(a.len(), 6);
    }

    #[test]
    fn an_array_parses_like_a_stream() {
        let lines: Vec<&str> = PROBE1_FOO.lines().collect();
        let arr = format!("[{}]", lines.join(","));
        let m = parse_matches(arr.as_bytes()).expect("parse");
        assert_eq!(m.len(), 10);
    }

    #[test]
    fn empty_output_is_no_matches_and_junk_is_an_error() {
        assert!(parse_matches(b"").expect("parse").is_empty());
        assert!(parse_matches(PROBE2_ZZZ.as_bytes()).expect("parse").is_empty());
        let err = parse_matches(b"not json\n").map(|_| ()).expect_err("junk");
        assert!(err.contains("did not parse"), "got: {err}");
    }

    #[test]
    fn real_output_becomes_edits_without_target_and_lifetimes() {
        let dir = tmp();
        project(&dir);
        let m = parse_matches(fixture_for(&dir).as_bytes()).expect("parse");
        let mut read = reader();
        let edits =
            edits_from(&m, &dir, "a", "b", &HiddenSet::empty(), &mut read).expect("edits");
        let names: Vec<String> = edits
            .iter()
            .map(|e| e.path.strip_prefix(&dir).expect("under dir").display().to_string())
            .collect();
        assert_eq!(names, vec!["ign/i.rs", "src/lib.rs", "src/sub/inner.rs"]);
        assert_eq!(edits[1].occurrences, 1);
        assert_eq!(edits[1].after, LIB_RS.replace("let a = 1", "let b = 1"));
        assert_eq!(edits[2].after, "fn b() {}\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_changed_file_fails_naming_it() {
        let dir = tmp();
        project(&dir);
        fs::write(dir.join("src/lib.rs"), "fn nothing_here() {}\n").expect("write");
        let m = parse_matches(fixture_for(&dir).as_bytes()).expect("parse");
        let mut read = reader();
        let err = edits_from(&m, &dir, "a", "b", &HiddenSet::empty(), &mut read)
            .expect_err("stale");
        assert!(err.contains("lib.rs"), "got: {err}");
        assert!(err.contains("is not `a`"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zero_matches_fail() {
        let m = parse_matches(PROBE2_ZZZ.as_bytes()).expect("parse");
        let mut read = reader();
        let err = edits_from(&m, Path::new("/tmp/x"), "zzz", "b", &HiddenSet::empty(), &mut read)
            .expect_err("none");
        assert!(err.contains("no identifier named"), "got: {err}");
    }

    #[test]
    fn overlapping_matches_fail() {
        let dir = tmp();
        let f = dir.join("a.rs");
        fs::write(&f, "fn foo() {}\n").expect("write");
        let lines = format!("{}{}", match_line(&f, 3, 6), match_line(&f, 5, 8));
        let m = parse_matches(lines.as_bytes()).expect("parse");
        let mut read = reader();
        let err = edits_from(&m, &dir, "foo", "bar", &HiddenSet::empty(), &mut read)
            .expect_err("overlap");
        assert!(err.contains("overlapping"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_outside_the_folder_fails() {
        let m = parse_matches(match_line(Path::new("/elsewhere/x.rs"), 0, 1).as_bytes())
            .expect("parse");
        let mut read = reader();
        let err = edits_from(&m, Path::new("/tmp/p"), "a", "b", &HiddenSet::empty(), &mut read)
            .expect_err("outside");
        assert!(err.contains("outside"), "got: {err}");
    }

    #[test]
    fn a_hidden_file_fails_naming_it() {
        let dir = tmp();
        project(&dir);
        let hidden = HiddenSet::from_lines(&[dir.join("src").display().to_string()]);
        let m = parse_matches(fixture_for(&dir).as_bytes()).expect("parse");
        let mut read = reader();
        let err = edits_from(&m, &dir, "a", "b", &hidden, &mut read).expect_err("hidden");
        assert!(
            err.starts_with("deliberately hidden by the user: "),
            "got: {err}"
        );
        assert!(err.contains("src"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn too_many_files_fail() {
        let mut lines = String::new();
        for i in 0..=MAX_FILES {
            lines.push_str(&match_line(Path::new(&format!("/tmp/p/f{i}.rs")), 0, 1));
        }
        let m = parse_matches(lines.as_bytes()).expect("parse");
        let mut read = reader();
        let err = edits_from(&m, Path::new("/tmp/p"), "a", "b", &HiddenSet::empty(), &mut read)
            .expect_err("cap");
        assert!(err.contains("limit is 200"), "got: {err}");
    }

    #[test]
    fn skipped_components() {
        assert!(skipped_component(Path::new("target/x.rs")));
        assert!(skipped_component(Path::new("src/.git/x.rs")));
        assert!(skipped_component(Path::new(".hidden/x.rs")));
        assert!(!skipped_component(Path::new("src/a.rs")));
        assert!(!skipped_component(Path::new(".env.rs")));
        assert!(!skipped_component(Path::new("src/target.rs")));
    }

    #[test]
    fn a_missing_binary_is_reported() {
        let dir = tmp();
        set_override(Some(dir.join("nope")));
        let err = find_binary().expect_err("missing");
        assert!(err.contains("not found"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_binary_without_the_flags_is_too_old() {
        let dir = tmp();
        fake_ast_grep(&dir, "ast-grep 0.1.0", "nothing useful", "exit 0");
        let err = find_binary().expect_err("old");
        assert!(err.contains("too old"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_binary_that_is_not_ast_grep_counts_as_missing() {
        let dir = tmp();
        fake_ast_grep(&dir, "sg 1.0", "--inline-rules --json", "exit 0");
        let err = find_binary().expect_err("not ast-grep");
        assert!(err.contains("not found"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_non_zero_exit_gives_one_line_with_stderr() {
        let dir = tmp();
        fake_ast_grep(
            &dir,
            "ast-grep 9.9.9",
            "--inline-rules --json",
            "echo 'boom: bad rule' >&2; echo second >&2; exit 2",
        );
        let mut read = reader();
        let err = plan(&dir, "a", "b", &HiddenSet::empty(), &mut read).expect_err("exit");
        assert!(err.contains("status 2"), "got: {err}");
        assert!(err.contains("boom: bad rule"), "got: {err}");
        assert!(!err.contains('\n'), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_slow_binary_is_killed() {
        let dir = tmp();
        let bin = fake_ast_grep(&dir, "ast-grep 9.9.9", "--inline-rules --json", "exec sleep 5");
        let started = Instant::now();
        let err = scan_with(&bin, &dir, "a", Duration::from_millis(1200))
            .map(|_| ())
            .expect_err("timeout");
        assert!(err.contains("took longer than"), "got: {err}");
        assert!(started.elapsed() < Duration::from_secs(4));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_healthy_fake_plans_the_rename() {
        let dir = tmp();
        let proj = dir.join("proj");
        fs::create_dir_all(&proj).expect("mkdir");
        let file = proj.join("a.rs");
        fs::write(&file, "fn foo() {}\n").expect("write");
        healthy(&dir, &match_line(&file, 3, 6));
        let mut read = reader();
        let edits =
            plan(&proj, "foo", "bar", &HiddenSet::empty(), &mut read).expect("plan");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].after, "fn bar() {}\n");
        let _ = fs::remove_dir_all(&dir);
    }
}
