//! Per-(path, content-hash) approval for editing `.sh` and `.toml` files.
//! Those extensions are treated as sensitive: even inside a whitelisted
//! folder, a chain that writes one goes to review. Once the user confirms
//! such a write, the file's content hash at approval time is stored, and
//! a later chain that touches the same path at the same content doesn't
//! re-ask.
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
    /// Hash of the file's content at approval time.
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

pub fn record_path(path: &Path) -> PathBuf {
    config_dir().join(format!("{}.json", key(path)))
}

pub fn load(path: &Path) -> Option<FileEntry> {
    let p = record_path(path);
    let text = std::fs::read_to_string(&p).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(entry: &FileEntry) -> Result<(), String> {
    let p = record_path(&entry.path);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    let text = serde_json::to_string_pretty(entry).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&p, text).map_err(|e| format!("write {}: {}", p.display(), e))
}

pub fn remove(path: &Path) -> Result<(), String> {
    let p = record_path(path);
    match std::fs::remove_file(&p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("remove {}: {}", p.display(), e)),
    }
}

/// True if the path has an extension we treat as sensitive.
pub fn is_sensitive(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => SENSITIVE_EXTS.iter().any(|s| s.eq_ignore_ascii_case(ext)),
        None => false,
    }
}

/// Sha of a path's current content, or the empty-string sha if the file
/// doesn't exist. Used to compare against the stored approval.
pub fn sha_of_current(path: &Path) -> Result<String, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(sha_of(&bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(sha_of(b"")),
        Err(e) => Err(format!("read {}: {}", path.display(), e)),
    }
}

/// True if there's a stored approval for this path and the file's content
/// hash still matches what was approved.
pub fn is_approved(path: &Path) -> bool {
    let Some(entry) = load(path) else {
        return false;
    };
    match sha_of_current(path) {
        Ok(now) => now == entry.sha,
        Err(_) => false,
    }
}

/// Every sensitive path in the list has a matching approval.
pub fn all_approved(paths: &[PathBuf]) -> bool {
    paths.iter().filter(|p| is_sensitive(p)).all(|p| is_approved(p))
}

/// Save an approval for each sensitive path, keyed on the file's content
/// right now. Called after a chain is confirmed through review, so the
/// user's approval of the write is remembered.
pub fn record_approved(paths: &[PathBuf]) -> Result<(), String> {
    for p in paths {
        if !is_sensitive(p) {
            continue;
        }
        let sha = sha_of_current(p)?;
        let entry = FileEntry {
            path: p.clone(),
            sha,
            at_secs: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        save(&entry)?;
    }
    Ok(())
}

/// Drop an approval. Called when a chain removes or renames a .sh/.toml
/// file, so a recreate at the same path re-asks.
pub fn remove_approval(path: &Path) {
    let _ = remove(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_extensions() {
        assert!(is_sensitive(Path::new("/x/a.sh")));
        assert!(is_sensitive(Path::new("/x/a.toml")));
        assert!(is_sensitive(Path::new("/x/A.SH")));
        assert!(!is_sensitive(Path::new("/x/a.rs")));
        assert!(!is_sensitive(Path::new("/x/a")));
    }
}
