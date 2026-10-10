//! A record of the last change zapgui made to each file. In memory only.
//! One record per path, the newest change only. Capped at 200 records.

use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

const MAX_CONTENT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_RECORDS: usize = 200;

#[derive(Clone, Debug)]
pub enum Before {
    Missing,
    Text(String),
    Skipped(u64),
}

#[derive(Clone, Debug)]
pub enum Kind {
    Edited,
    Created,
    Removed,
    RenamedFrom(PathBuf),
    RenamedTo(PathBuf),
}

#[derive(Clone, Debug)]
pub struct Record {
    pub before: Before,
    pub kind: Kind,
    pub after_hash: Option<u64>,
    pub at: SystemTime,
}

static RECORDS: Mutex<Vec<(PathBuf, Record)>> = Mutex::new(Vec::new());

pub fn key(path: &Path) -> PathBuf {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    if let Ok(canon) = out.canonicalize() {
        return canon;
    }
    out
}

pub fn classify_content(s: &str) -> Before {
    let bytes = s.as_bytes().len() as u64;
    if bytes > MAX_CONTENT_BYTES {
        Before::Skipped(bytes)
    } else {
        Before::Text(s.to_string())
    }
}

pub fn hash_text(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub fn record(path: &Path, before: Before, kind: Kind, after: Option<&str>) {
    let key = key(path);
    let rec = Record {
        before,
        kind,
        after_hash: after.map(hash_text),
        at: SystemTime::now(),
    };
    let Ok(mut records) = RECORDS.lock() else { return };
    if let Some(pos) = records.iter().position(|(p, _)| p == &key) {
        records.remove(pos);
    }
    records.push((key, rec));
    if records.len() > MAX_RECORDS {
        let drop = records.len() - MAX_RECORDS;
        records.drain(0..drop);
    }
}

pub fn lookup(path: &Path) -> Option<Record> {
    let key = key(path);
    let records = RECORDS.lock().ok()?;
    records
        .iter()
        .rev()
        .find(|(p, _)| p == &key)
        .map(|(_, r)| r.clone())
}

pub fn most_recent() -> Option<PathBuf> {
    let records = RECORDS.lock().ok()?;
    records.last().map(|(p, _)| p.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_resolves_relative_to_absolute() {
        let a = key(Path::new("/tmp/x"));
        let b = key(Path::new("/tmp/./x"));
        assert_eq!(a, b);
    }

    #[test]
    fn eviction_drops_oldest() {
        let _ = RECORDS.lock().map(|mut r| r.clear());
        for i in 0..(MAX_RECORDS + 5) {
            let p = PathBuf::from(format!("/tmp/zap_last_test_{i}"));
            record(&p, Before::Missing, Kind::Created, Some("x"));
        }
        let len = RECORDS.lock().map(|r| r.len()).unwrap_or(0);
        assert!(len <= MAX_RECORDS);
    }

    #[test]
    fn classify_skips_huge() {
        let big = "a".repeat((MAX_CONTENT_BYTES + 1) as usize);
        matches!(classify_content(&big), Before::Skipped(_));
    }
}
