//! Trusted "run this file" entries. One per (canonical path, exact args).
//! Stored under ~/.config/zap/run_trust/, keyed by a hash of the pair.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunEntry {
    pub path: PathBuf,
    pub args: Vec<String>,
    pub sha: String,
    /// The file text at approval time, if it was UTF-8 and under 1 MiB.
    /// Empty when the file was binary or too big.
    pub text: String,
    pub at_secs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Trusted,
    Review { old_text: Option<String> },
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });
    base.join("zap").join("run_trust")
}

pub fn key(path: &Path, args: &[String]) -> String {
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    for a in args {
        a.hash(&mut h);
    }
    format!("{:016x}", h.finish())
}

pub fn record_path(path: &Path, args: &[String]) -> PathBuf {
    config_dir().join(format!("{}.json", key(path, args)))
}

pub fn load(path: &Path, args: &[String]) -> Option<RunEntry> {
    let p = record_path(path, args);
    let text = std::fs::read_to_string(&p).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(entry: &RunEntry) -> Result<(), String> {
    let p = record_path(&entry.path, &entry.args);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    let text = serde_json::to_string_pretty(entry).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&p, text).map_err(|e| format!("write {}: {}", p.display(), e))
}

pub fn remove(path: &Path, args: &[String]) -> Result<(), String> {
    let p = record_path(path, args);
    match std::fs::remove_file(&p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("remove {}: {}", p.display(), e)),
    }
}

pub fn sha_of(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Read the file and produce (sha, text_if_utf8_and_small).
/// `text` is empty when the file is binary or over 1 MiB.
pub fn read_for_entry(path: &Path) -> Result<(String, String), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let sha = sha_of(&bytes);
    let text = if bytes.len() <= 1024 * 1024 {
        match std::str::from_utf8(&bytes) {
            Ok(s) if !s.as_bytes().contains(&0) => s.to_string(),
            _ => String::new(),
        }
    } else {
        String::new()
    };
    Ok((sha, text))
}

pub fn decide(entry: Option<&RunEntry>, now_sha: &str) -> Decision {
    match entry {
        Some(e) if e.sha == now_sha => Decision::Trusted,
        Some(e) => {
            let old = if e.text.is_empty() { None } else { Some(e.text.clone()) };
            Decision::Review { old_text: old }
        }
        None => Decision::Review { old_text: None },
    }
}

/// Re-key every entry whose path matches `from`, keeping sha/text/at.
/// Args are preserved as they were.
pub fn rename(from: &Path, to: &Path) {
    let dir = config_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    let mut moved: Vec<(PathBuf, RunEntry)> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        let Ok(text) = std::fs::read_to_string(&p) else { continue };
        let Ok(mut entry): Result<RunEntry, _> = serde_json::from_str(&text) else { continue };
        if entry.path == from {
            entry.path = to.to_path_buf();
            moved.push((p, entry));
        }
    }
    for (old_path, entry) in moved {
        let new_path = record_path(&entry.path, &entry.args);
        let _ = std::fs::remove_file(&old_path);
        if let Ok(text) = serde_json::to_string_pretty(&entry) {
            let _ = std::fs::write(&new_path, text);
        }
    }
}

/// True if any stored entry has this path (any args).
pub fn any_entry_for_path(path: &Path) -> bool {
    let dir = config_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else { return false };
    for e in entries.flatten() {
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let Ok(entry): Result<RunEntry, _> = serde_json::from_str(&text) else { continue };
        if entry.path == path {
            return true;
        }
    }
    false
}

/// A one-line note for the RENAME REQUEST caption, if the file at this
/// path is currently trusted for running (either as a run target or as a
/// file in a trusted cargo surface).
pub fn trust_note(path: &Path) -> Option<String> {
    if any_entry_for_path(path) {
        return Some("trusted for running".to_string());
    }
    if crate::trust::any_record_contains(path) {
        return Some("trusted for running".to_string());
    }
    None
}

/// Drop every entry whose path matches `path`.
pub fn remove_all_at_path(path: &Path) {
    let dir = config_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let Ok(text) = std::fs::read_to_string(&p) else { continue };
        let Ok(entry): Result<RunEntry, _> = serde_json::from_str(&text) else { continue };
        if entry.path == path {
            let _ = std::fs::remove_file(&p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_trusted_on_identical_sha() {
        let entry = RunEntry {
            path: PathBuf::from("/x.sh"),
            args: vec![],
            sha: "abc".into(),
            text: "hi".into(),
            at_secs: 0,
        };
        assert_eq!(decide(Some(&entry), "abc"), Decision::Trusted);
    }

    #[test]
    fn decide_review_on_changed_sha() {
        let entry = RunEntry {
            path: PathBuf::from("/x.sh"),
            args: vec![],
            sha: "abc".into(),
            text: "hi".into(),
            at_secs: 0,
        };
        match decide(Some(&entry), "def") {
            Decision::Review { old_text } => assert_eq!(old_text.as_deref(), Some("hi")),
            _ => panic!("expected review"),
        }
    }

    #[test]
    fn decide_review_on_missing_entry() {
        match decide(None, "abc") {
            Decision::Review { old_text } => assert!(old_text.is_none()),
            _ => panic!("expected review"),
        }
    }

    #[test]
    fn sha_of_empty_is_known() {
        assert_eq!(
            sha_of(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
