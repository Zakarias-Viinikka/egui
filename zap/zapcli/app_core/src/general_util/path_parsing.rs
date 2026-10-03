use std::path::{Path, PathBuf};

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Turn a user-typed path into an absolute PathBuf.
/// `~` and `~/...` expand to $HOME. Absolute paths pass through. Relative
/// paths are joined against the session's current directory.
pub fn resolve(path: &str, cwd: &Path) -> PathBuf {
    if path == "~" {
        return home_dir();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        cwd.join(p)
    }
}
