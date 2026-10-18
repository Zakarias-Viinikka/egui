use super::cache::{cache_dir, LIMIT_BYTES};
use std::fs;

struct Entry {
    path: std::path::PathBuf,
    size: u64,
    mtime: std::time::SystemTime,
}

pub fn evict_if_over(limit_bytes: u64) {
    let dir = cache_dir();
    let Ok(read) = fs::read_dir(&dir) else { return };
    let mut entries: Vec<Entry> = Vec::new();
    let mut total: u64 = 0;
    for e in read.flatten() {
        let Ok(meta) = e.metadata() else { continue };
        if !meta.is_file() { continue; }
        let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
        total += meta.len();
        entries.push(Entry { path: e.path(), size: meta.len(), mtime });
    }
    if total <= limit_bytes { return; }
    entries.sort_by_key(|e| e.mtime);
    for entry in entries {
        if total <= limit_bytes { break; }
        if fs::remove_file(&entry.path).is_ok() { total = total.saturating_sub(entry.size); }
    }
}

pub fn evict_default() { evict_if_over(LIMIT_BYTES); }
