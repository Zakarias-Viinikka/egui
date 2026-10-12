use std::path::{Path, PathBuf};

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Turn a user-typed path into an absolute PathBuf.
/// `~`, `~/...`, `$HOME`, `&HOME` and their `/...` forms expand to $HOME.
/// Absolute paths pass through. Relative paths are joined against the
/// session's current directory.
pub fn resolve(path: &str, cwd: &Path) -> PathBuf {
    if path == "~" || path == "$HOME" || path == "&HOME" {
        return home_dir();
    }
    for prefix in ["~/", "$HOME/", "&HOME/"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            return home_dir().join(rest);
        }
    }
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        cwd.join(p)
    }
}