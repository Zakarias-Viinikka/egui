use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });
    base.join("zap").join("whitelist.txt")
}

pub fn load() -> Vec<PathBuf> {
    let path = config_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        out.push(PathBuf::from(line));
    }
    out
}

pub fn save(folders: &[PathBuf]) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    let mut text = String::new();
    for f in folders {
        text.push_str(&f.display().to_string());
        text.push('\n');
    }
    std::fs::write(&path, text).map_err(|e| format!("write {}: {}", path.display(), e))
}

/// Canonicalize and refuse paths that would defeat the point. Accepts a
/// regular file or a folder; a hidden path is refused.
pub fn validate_add(path: &Path) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("not a file or folder: {}", e))?;
    if !canonical.is_dir() && !canonical.is_file() {
        return Err("not a file or folder".to_string());
    }
    if canonical == Path::new("/") {
        return Err("can't whitelist /".to_string());
    }
    let home = std::env::var("HOME").ok().map(PathBuf::from);
    if let Some(h) = home {
        if let Ok(hc) = h.canonicalize() {
            if canonical == hc {
                return Err("can't whitelist home itself".to_string());
            }
        }
    }
    if let Some(msg) = zap_wire::hidden::HiddenSet::check(&canonical) {
        return Err(msg);
    }
    Ok(canonical)
}

/// True if `file` matches any whitelist entry.
///
/// A regular-file entry matches only that file. A folder entry (or one
/// that does not exist yet) matches by whole components, as before.
pub fn is_whitelisted(file: &Path, entries: &[PathBuf]) -> bool {
    let resolved_file = resolve_existing_parent(file);
    for entry in entries {
        let resolved_entry = resolve_existing_parent(entry);
        if resolved_entry.is_file() {
            if resolved_file == resolved_entry {
                return true;
            }
        } else if starts_with_components(&resolved_file, &resolved_entry) {
            return true;
        }
    }
    false
}

/// Kept for the callers that have not switched yet.
pub fn is_inside(file: &Path, entries: &[PathBuf]) -> bool {
    is_whitelisted(file, entries)
}

fn resolve_existing_parent(path: &Path) -> PathBuf {
    if let Ok(canon) = path.canonicalize() {
        return canon;
    }
    let mut tail: Vec<OsString> = Vec::new();
    let mut current = path.to_path_buf();
    loop {
        let Some(name) = current.file_name() else {
            return path.to_path_buf();
        };
        tail.push(name.to_os_string());
        let Some(parent) = current.parent() else {
            return path.to_path_buf();
        };
        if let Ok(canon) = parent.canonicalize() {
            let mut out = canon;
            for name in tail.iter().rev() {
                out.push(name);
            }
            return out;
        }
        current = parent.to_path_buf();
    }
}

fn starts_with_components(path: &Path, prefix: &Path) -> bool {
    let mut p = path.components();
    for c in prefix.components() {
        match p.next() {
            Some(pc) if pc == c => {}
            _ => return false,
        }
    }
    true
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
        p.push(format!("zap_whitelist_test_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    #[test]
    fn file_entry_matches_only_itself() {
        let dir = tmp();
        let file = dir.join("a.rs");
        let sibling = dir.join("b.rs");
        let below = dir.join("a.rs").join("child.rs");
        fs::write(&file, "x").expect("write");
        fs::write(&sibling, "x").expect("write");
        let entries = vec![file.clone()];
        assert!(is_whitelisted(&file, &entries));
        assert!(!is_whitelisted(&sibling, &entries));
        assert!(!is_whitelisted(&below, &entries));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_entry_matches_below() {
        let dir = tmp();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).expect("mkdir");
        let f = sub.join("a.rs");
        fs::write(&f, "x").expect("write");
        let entries = vec![sub.clone()];
        assert!(is_whitelisted(&f, &entries));
        assert!(is_whitelisted(&sub, &entries));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_add_accepts_a_file() {
        let dir = tmp();
        let f = dir.join("a.rs");
        fs::write(&f, "x").expect("write");
        let canon = validate_add(&f).expect("accept");
        assert!(canon.is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_add_refuses_root_and_home() {
        assert!(validate_add(Path::new("/")).is_err());
        let home = std::env::var("HOME").ok().map(PathBuf::from);
        if let Some(h) = home {
            if h.exists() {
                let err = validate_add(&h).expect_err("refuse home");
                assert!(err.contains("home"), "got: {err}");
            }
        }
    }
}

pub fn hidden_path() -> PathBuf {
    zap_wire::config::config_dir().join("hidden.txt")
}

pub fn load_hidden() -> Vec<String> {
    zap_wire::config::read_lines(&hidden_path()).unwrap_or_default()
}

pub fn save_hidden(lines: &[String]) -> Result<(), String> {
    zap_wire::config::write_lines(&hidden_path(), lines).map_err(|e| e.to_string())
}

/// Refuse entries that would defeat the point. Accepts any absolute path,
/// any ~/path, and any bare name. A path with `..` in it, `/`, `~`, `~/`
/// and the home folder itself are refused.
pub fn validate_hidden_entry(raw: &str) -> Result<String, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("empty entry".to_string());
    }
    if t == "/" {
        return Err("can't hide /".to_string());
    }
    if t == "~" || t == "~/" {
        return Err("can't hide the home folder".to_string());
    }
    if t.starts_with('/') {
        let p = Path::new(t);
        if p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("path contains ..".to_string());
        }
        if let Ok(home) = std::env::var("HOME") {
            if p == PathBuf::from(home) {
                return Err("can't hide the home folder".to_string());
            }
        }
        return Ok(t.to_string());
    }
    if let Some(rest) = t.strip_prefix("~/") {
        if rest.is_empty() {
            return Err("can't hide the home folder".to_string());
        }
        if rest.split('/').any(|c| c == "..") {
            return Err("path contains ..".to_string());
        }
        return Ok(t.to_string());
    }
    if t.contains('/') {
        return Err("use an absolute path, ~/path, or a bare name".to_string());
    }
    Ok(t.to_string())
}
