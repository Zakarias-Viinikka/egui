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

/// Canonicalize and refuse folders that would defeat the point.
pub fn validate_add(path: &Path) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("not a folder: {}", e))?;
    if !canonical.is_dir() {
        return Err("not a folder".to_string());
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
    Ok(canonical)
}

pub fn is_inside(file: &Path, folders: &[PathBuf]) -> bool {
    let resolved_file = resolve_existing_parent(file);
    for folder in folders {
        let resolved_folder = resolve_existing_parent(folder);
        if starts_with_components(&resolved_file, &resolved_folder) {
            return true;
        }
    }
    false
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
