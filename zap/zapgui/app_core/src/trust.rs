//! Trusted execution surface per project. Stored under ~/.config/zap/trust/,
//! one record per project root. A run is allowed if the surface on disk
//! matches what's stored; anything different means the surface is shown for
//! review and only stored on confirm.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::surface::SurfaceFile;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustedFile {
    pub path: PathBuf,
    pub hash: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustedRecord {
    pub root: PathBuf,
    pub files: Vec<TrustedFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceDiff {
    pub path: PathBuf,
    pub old_text: Option<String>,
    pub new_text: String,
}

pub fn hash_text(s: &str) -> String {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });
    base.join("zap").join("trust")
}

pub fn record_path(root: &Path) -> PathBuf {
    let name = hash_text(&root.to_string_lossy());
    config_dir().join(format!("{name}.json"))
}

pub fn load(root: &Path) -> Option<TrustedRecord> {
    let path = record_path(root);
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(record: &TrustedRecord) -> Result<(), String> {
    let path = record_path(&record.root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    let text = serde_json::to_string_pretty(record)
        .map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&path, text).map_err(|e| format!("write {}: {}", path.display(), e))
}

pub fn from_surface(root: &Path, surface: &[SurfaceFile]) -> TrustedRecord {
    TrustedRecord {
        root: root.to_path_buf(),
        files: surface
            .iter()
            .map(|f| TrustedFile {
                path: f.path.clone(),
                hash: hash_text(&f.text),
                text: f.text.clone(),
            })
            .collect(),
    }
}

pub fn compare(record: &TrustedRecord, surface: &[SurfaceFile]) -> Vec<SurfaceDiff> {
    let mut diffs: Vec<SurfaceDiff> = Vec::new();

    let by_path: std::collections::HashMap<&Path, &TrustedFile> = record
        .files
        .iter()
        .map(|f| (f.path.as_path(), f))
        .collect();

    for f in surface {
        match by_path.get(f.path.as_path()) {
            Some(trusted) => {
                if trusted.hash != hash_text(&f.text) {
                    diffs.push(SurfaceDiff {
                        path: f.path.clone(),
                        old_text: Some(trusted.text.clone()),
                        new_text: f.text.clone(),
                    });
                }
            }
            None => diffs.push(SurfaceDiff {
                path: f.path.clone(),
                old_text: None,
                new_text: f.text.clone(),
            }),
        }
    }

    let on_disk: std::collections::HashSet<&Path> =
        surface.iter().map(|f| f.path.as_path()).collect();
    for tf in &record.files {
        if !on_disk.contains(tf.path.as_path()) {
            diffs.push(SurfaceDiff {
                path: tf.path.clone(),
                old_text: Some(tf.text.clone()),
                new_text: String::new(),
            });
        }
    }

    diffs.sort_by(|a, b| a.path.cmp(&b.path));
    diffs
}

pub fn refresh_lock(record: &mut TrustedRecord, surface: &[SurfaceFile]) {
    for f in surface {
        if f.path.file_name().map(|n| n == "Cargo.lock").unwrap_or(false) {
            if let Some(tf) = record.files.iter_mut().find(|t| t.path == f.path) {
                tf.hash = hash_text(&f.text);
                tf.text = f.text.clone();
            }
        }
    }
}

pub fn refresh_after_chain_confirm(
    record: &mut TrustedRecord,
    written: &[(PathBuf, String)],
) -> bool {
    let mut changed = false;
    for (path, text) in written {
        if let Some(tf) = record.files.iter_mut().find(|t| &t.path == path) {
            tf.hash = hash_text(text);
            tf.text = text.clone();
            changed = true;
        }
    }
    changed
}

/// Re-key a file entry if `from` is in this record. Used when a Move step
/// is confirmed with the destination inside the same project.
pub fn rename_in_record(root: &Path, from: &Path, to: &Path) {
    let Some(mut record) = load(root) else { return };
    let mut changed = false;
    for f in record.files.iter_mut() {
        if f.path == from {
            f.path = to.to_path_buf();
            changed = true;
        }
    }
    if changed {
        let _ = save(&record);
    }
}

/// Drop a file entry from the record for `root`, if present.
pub fn remove_from_record(root: &Path, path: &Path) {
    let Some(mut record) = load(root) else { return };
    let before = record.files.len();
    record.files.retain(|f| f.path != path);
    if record.files.len() != before {
        let _ = save(&record);
    }
}

/// True if any stored project record has this file in its surface.
pub fn any_record_contains(path: &Path) -> bool {
    let dir = config_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else { return false };
    for e in entries.flatten() {
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let Ok(record): Result<TrustedRecord, _> = serde_json::from_str(&text) else { continue };
        if record.files.iter().any(|f| f.path == path) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(entries: &[(&str, &str)]) -> Vec<SurfaceFile> {
        entries
            .iter()
            .map(|(p, t)| SurfaceFile {
                path: PathBuf::from(p),
                text: (*t).to_string(),
            })
            .collect()
    }

    #[test]
    fn identical_surface_has_no_diffs() {
        let s = surface(&[("/a/Cargo.toml", "[package]\n")]);
        let record = from_surface(Path::new("/a"), &s);
        assert!(compare(&record, &s).is_empty());
    }

    #[test]
    fn changed_file_shows_as_diff() {
        let s1 = surface(&[("/a/Cargo.toml", "one")]);
        let record = from_surface(Path::new("/a"), &s1);
        let s2 = surface(&[("/a/Cargo.toml", "two")]);
        let diffs = compare(&record, &s2);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].old_text.as_deref(), Some("one"));
        assert_eq!(diffs[0].new_text, "two");
    }

    #[test]
    fn new_file_shows_without_old() {
        let s1 = surface(&[("/a/Cargo.toml", "x")]);
        let record = from_surface(Path::new("/a"), &s1);
        let s2 = surface(&[("/a/Cargo.toml", "x"), ("/a/build.rs", "fn main(){}")]);
        let diffs = compare(&record, &s2);
        assert_eq!(diffs.len(), 1);
        assert!(diffs[0].old_text.is_none());
    }

    #[test]
    fn removed_file_shows_with_empty_new() {
        let s1 = surface(&[("/a/Cargo.toml", "x"), ("/a/build.rs", "y")]);
        let record = from_surface(Path::new("/a"), &s1);
        let s2 = surface(&[("/a/Cargo.toml", "x")]);
        let diffs = compare(&record, &s2);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].new_text, "");
    }

    #[test]
    fn refresh_lock_updates_only_lock() {
        let s = surface(&[("/a/Cargo.toml", "man"), ("/a/Cargo.lock", "lock-v1")]);
        let mut record = from_surface(Path::new("/a"), &s);
        let s2 = surface(&[("/a/Cargo.toml", "man"), ("/a/Cargo.lock", "lock-v2")]);
        refresh_lock(&mut record, &s2);
        assert!(compare(&record, &s2).is_empty());
    }

    #[test]
    fn chain_confirm_refreshes_only_matching_files() {
        let s = surface(&[("/a/Cargo.toml", "man"), ("/a/src/lib.rs", "src")]);
        let mut record = from_surface(Path::new("/a"), &s);
        let written = vec![
            (PathBuf::from("/a/Cargo.toml"), "new-man".to_string()),
            (PathBuf::from("/other/file.rs"), "ignore".to_string()),
        ];
        assert!(refresh_after_chain_confirm(&mut record, &written));
        let s2 = surface(&[("/a/Cargo.toml", "new-man"), ("/a/src/lib.rs", "src")]);
        assert!(compare(&record, &s2).is_empty());
    }
}
