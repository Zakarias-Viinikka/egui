//! Per-(path, content-hash) approval for editing `.sh` and `.toml` files.
//!
//! The key is "this exact content may be written at this path". An entry
//! stores the path, the hash of the text a chain would leave in the file,
//! and a time. The check runs before the write: it compares the stored
//! hash with the hash of the text the chain is about to write, and first
//! applies the drift rule - if the file on disk no longer hashes to the
//! stored value, something other than zap changed it, so the entry is
//! deleted and the answer is false.
//!
//! Shape mirrors run_trust.rs. One file per path under
//! ~/.config/zap/file_trust/, keyed by a hash of the path.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::run_trust::sha_of;

const SENSITIVE_EXTS: &[&str] = &["sh", "toml"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    /// Hash of the text the chain writes at this path.
    pub sha: String,
    pub at_secs: u64,
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });
    base.join("zap").join("file_trust")
}

pub fn key(path: &Path) -> String {
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn record_path_in(dir: &Path, path: &Path) -> PathBuf {
    dir.join(format!("{}.json", key(path)))
}

pub fn record_path(path: &Path) -> PathBuf {
    record_path_in(&config_dir(), path)
}

pub fn load_in(dir: &Path, path: &Path) -> Option<FileEntry> {
    let p = record_path_in(dir, path);
    let text = std::fs::read_to_string(&p).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn load(path: &Path) -> Option<FileEntry> {
    load_in(&config_dir(), path)
}

pub fn save_in(dir: &Path, entry: &FileEntry) -> Result<(), String> {
    let p = record_path_in(dir, &entry.path);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    let text = serde_json::to_string_pretty(entry).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&p, text).map_err(|e| format!("write {}: {}", p.display(), e))
}

pub fn save(entry: &FileEntry) -> Result<(), String> {
    save_in(&config_dir(), entry)
}

pub fn remove_in(dir: &Path, path: &Path) -> Result<(), String> {
    let p = record_path_in(dir, path);
    match std::fs::remove_file(&p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("remove {}: {}", p.display(), e)),
    }
}

pub fn remove(path: &Path) -> Result<(), String> {
    remove_in(&config_dir(), path)
}

/// True if the path has an extension we treat as sensitive.
pub fn is_sensitive(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => SENSITIVE_EXTS.iter().any(|s| s.eq_ignore_ascii_case(ext)),
        None => false,
    }
}

/// Sha of a path's current content, or the empty-string sha if the file
/// doesn't exist. Used by the drift rule only.
pub fn sha_of_current(path: &Path) -> Result<String, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(sha_of(&bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(sha_of(b"")),
        Err(e) => Err(format!("read {}: {}", path.display(), e)),
    }
}

/// True if the chain's write of `write_text` at `path` is approved.
///
/// First applies the drift rule: if the file on disk no longer hashes to
/// the stored value, something other than zap changed it after the
/// approval, so the entry is deleted on the spot and the answer is false.
/// Otherwise the answer is true only when the stored hash equals the hash
/// of the write.
pub fn is_approved_in(dir: &Path, path: &Path, write_text: &str) -> bool {
    let Some(entry) = load_in(dir, path) else {
        return false;
    };
    let now = match sha_of_current(path) {
        Ok(h) => h,
        Err(_) => return false,
    };
    if now != entry.sha {
        let _ = remove_in(dir, path);
        return false;
    }
    sha_of(write_text.as_bytes()) == entry.sha
}

pub fn is_approved(path: &Path, write_text: &str) -> bool {
    is_approved_in(&config_dir(), path, write_text)
}

/// Every sensitive write in the list has a matching approval.
pub fn all_approved_in(dir: &Path, writes: &[(PathBuf, String)]) -> bool {
    writes
        .iter()
        .filter(|(p, _)| is_sensitive(p))
        .all(|(p, t)| is_approved_in(dir, p, t))
}

pub fn all_approved(writes: &[(PathBuf, String)]) -> bool {
    all_approved_in(&config_dir(), writes)
}

/// Save an approval for each sensitive write, keyed on the hash of the
/// text the chain writes at that path. A step that was not written is not
/// in the list, so it records nothing.
pub fn record_approved_in(dir: &Path, writes: &[(PathBuf, String)]) -> Result<(), String> {
    for (p, text) in writes {
        if !is_sensitive(p) {
            continue;
        }
        let entry = FileEntry {
            path: p.clone(),
            sha: sha_of(text.as_bytes()),
            at_secs: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        save_in(dir, &entry)?;
    }
    Ok(())
}

pub fn record_approved(writes: &[(PathBuf, String)]) -> Result<(), String> {
    record_approved_in(&config_dir(), writes)
}

/// Drop an approval. Called when a chain removes or renames a .sh/.toml
/// file, so a recreate at the same path re-asks.
pub fn remove_approval(path: &Path) {
    let _ = remove(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("zap_file_trust_test_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    #[test]
    fn sensitive_extensions() {
        assert!(is_sensitive(Path::new("/x/a.sh")));
        assert!(is_sensitive(Path::new("/x/a.toml")));
        assert!(is_sensitive(Path::new("/x/A.SH")));
        assert!(!is_sensitive(Path::new("/x/a.rs")));
        assert!(!is_sensitive(Path::new("/x/a")));
    }

    #[test]
    fn no_entry_is_false() {
        let dir = tmp();
        let trust = dir.join("trust");
        let file = dir.join("a.sh");
        assert!(!is_approved_in(&trust, &file, "hello\n"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn recorded_write_is_approved_for_that_hash_only() {
        let dir = tmp();
        let trust = dir.join("trust");
        let file = dir.join("a.sh");
        fs::write(&file, "hello\n").expect("write");
        record_approved_in(&trust, &[(file.clone(), "hello\n".to_string())]).expect("record");
        assert!(is_approved_in(&trust, &file, "hello\n"));
        assert!(!is_approved_in(&trust, &file, "goodbye\n"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn out_of_band_change_revokes_and_stays_revoked() {
        let dir = tmp();
        let trust = dir.join("trust");
        let file = dir.join("a.sh");
        fs::write(&file, "hello\n").expect("write");
        record_approved_in(&trust, &[(file.clone(), "hello\n".to_string())]).expect("record");
        fs::write(&file, "tampered\n").expect("write");
        assert!(!is_approved_in(&trust, &file, "hello\n"));
        assert!(load_in(&trust, &file).is_none());
        fs::write(&file, "hello\n").expect("write");
        assert!(!is_approved_in(&trust, &file, "hello\n"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn deleted_file_revokes() {
        let dir = tmp();
        let trust = dir.join("trust");
        let file = dir.join("a.sh");
        fs::write(&file, "hello\n").expect("write");
        record_approved_in(&trust, &[(file.clone(), "hello\n".to_string())]).expect("record");
        fs::remove_file(&file).expect("remove");
        assert!(!is_approved_in(&trust, &file, "hello\n"));
        assert!(load_in(&trust, &file).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_approved_skips_non_sensitive_and_hashes_the_text() {
        let dir = tmp();
        let trust = dir.join("trust");
        let sh = dir.join("a.sh");
        let rs = dir.join("a.rs");
        fs::write(&rs, "fn main() {}\n").expect("write");
        record_approved_in(
            &trust,
            &[
                (sh.clone(), "hello\n".to_string()),
                (rs.clone(), "fn main() {}\n".to_string()),
            ],
        )
        .expect("record");
        assert!(load_in(&trust, &sh).is_some());
        assert!(load_in(&trust, &rs).is_none());
        let entry = load_in(&trust, &sh).expect("entry");
        assert_eq!(entry.sha, sha_of(b"hello\n"));
        let _ = fs::remove_dir_all(&dir);
    }
}
