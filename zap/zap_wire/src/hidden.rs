//! Paths the user deliberately hides from the AI. The set is read from
//! hidden.txt in the config folder. Matching is by whole path components,
//! never by string prefix. Hidden always wins over the whitelist.

use std::path::{Component, Path, PathBuf};

use crate::config::{config_dir, read_lines};

/// A hidden entry is one of two shapes.
#[derive(Clone, Debug)]
enum Entry {
    /// A path (starting with / or ~/). Hides itself and everything below.
    Path { lexical: PathBuf, resolved: PathBuf },
    /// A bare name with no slash. Matches any path with that component.
    Name(String),
}

#[derive(Clone, Debug, Default)]
pub struct HiddenSet {
    entries: Vec<Entry>,
}

impl HiddenSet {
    pub fn empty() -> Self {
        Self { entries: Vec::new() }
    }

    /// Build from a list of lines. Each line is one entry.
    pub fn from_lines(lines: &[String]) -> Self {
        let home = std::env::var("HOME").ok().map(PathBuf::from);
        let mut entries = Vec::new();
        for line in lines {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with('/') || t.starts_with("~/") {
                let lexical = lexical_form(t, home.as_deref());
                let resolved = resolve_existing_parent(&lexical);
                entries.push(Entry::Path { lexical, resolved });
            } else if !t.contains('/') {
                entries.push(Entry::Name(t.to_string()));
            }
            // Anything else (relative with slash, bare words with slash)
            // is ignored as a malformed entry.
        }
        Self { entries }
    }

    /// Load from hidden.txt in the config folder.
    pub fn load() -> Self {
        let path = config_dir().join("hidden.txt");
        match read_lines(&path) {
            Ok(lines) => Self::from_lines(&lines),
            Err(_) => Self::empty(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// True if the path is hidden. Both the lexical form and the resolved
    /// form are checked, so a symlink into a hidden folder and a `..` path
    /// that lands there are both caught.
    pub fn is_hidden(&self, path: &Path) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let home = std::env::var("HOME").ok().map(PathBuf::from);
        let lexical = lexical_form_path(path, home.as_deref());
        let resolved = resolve_existing_parent(path);
        for entry in &self.entries {
            match entry {
                Entry::Path { lexical: el, resolved: er } => {
                    if under(&lexical, el) || under(&resolved, er) {
                        return true;
                    }
                }
                Entry::Name(name) => {
                    if has_component(&lexical, name) || has_component(&resolved, name) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// The refusal line for a hidden path, built from the path as the
    /// caller wrote it made absolute. None if the path is not hidden.
    pub fn refusal(&self, path: &Path) -> Option<String> {
        if !self.is_hidden(path) {
            return None;
        }
        let home = std::env::var("HOME").ok().map(PathBuf::from);
        let shown = lexical_form_path(path, home.as_deref());
        Some(format!("deliberately hidden by the user: {}", shown.display()))
    }

    /// Convenience: load and check.
    pub fn check(path: &Path) -> Option<String> {
        Self::load().refusal(path)
    }
}

fn lexical_form(s: &str, home: Option<&Path>) -> PathBuf {
    let p = if s == "~" {
        home.map(|h| h.to_path_buf()).unwrap_or_else(|| PathBuf::from("~"))
    } else if let Some(rest) = s.strip_prefix("~/") {
        home.map(|h| h.join(rest)).unwrap_or_else(|| PathBuf::from(s))
    } else {
        PathBuf::from(s)
    };
    lexical_normalize(&p)
}

fn lexical_form_path(p: &Path, home: Option<&Path>) -> PathBuf {
    let s = p.to_string_lossy();
    if s.starts_with("~/") || s == "~" {
        return lexical_form(&s, home);
    }
    lexical_normalize(p)
}

/// Collapse . and .. without touching the disk.
fn lexical_normalize(p: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.last(), Some(Component::Normal(_))) {
                    out.pop();
                } else {
                    out.push(c);
                }
            }
            other => out.push(other),
        }
    }
    let mut buf = PathBuf::new();
    for c in out {
        buf.push(c.as_os_str());
    }
    buf
}

/// Resolve the nearest existing ancestor and append the remaining
/// components literally.
fn resolve_existing_parent(path: &Path) -> PathBuf {
    if let Ok(canon) = path.canonicalize() {
        return canon;
    }
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
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

fn under(path: &Path, prefix: &Path) -> bool {
    let mut p = path.components();
    for c in prefix.components() {
        match p.next() {
            Some(pc) if pc == c => {}
            _ => return false,
        }
    }
    true
}

fn has_component(path: &Path, name: &str) -> bool {
    path.components().any(|c| {
        matches!(c, Component::Normal(n) if n.to_string_lossy() == name)
    })
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
        p.push(format!("zap_hidden_test_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    fn set(entries: &[&str]) -> HiddenSet {
        HiddenSet::from_lines(&entries.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn file_entry_hides_exactly_that_file() {
        let dir = tmp();
        let f = dir.join("a.txt");
        fs::write(&f, "x").expect("write");
        let h = set(&[f.to_str().unwrap()]);
        assert!(h.is_hidden(&f));
        assert!(!h.is_hidden(&dir.join("b.txt")));
        assert!(!h.is_hidden(&dir.join("sub").join("a.txt")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_entry_hides_below_any_depth() {
        let dir = tmp();
        let sub = dir.join("secrets");
        fs::create_dir_all(sub.join("deep")).expect("mkdir");
        let h = set(&[sub.to_str().unwrap()]);
        assert!(h.is_hidden(&sub));
        assert!(h.is_hidden(&sub.join("deep").join("x.txt")));
        assert!(!h.is_hidden(&dir.join("public").join("x.txt")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bare_name_hides_any_component() {
        let h = set(&[".env"]);
        assert!(h.is_hidden(Path::new("/a/.env")));
        assert!(h.is_hidden(Path::new("/a/b/.env")));
        assert!(!h.is_hidden(Path::new("/a/.envrc")));
        assert!(h.is_hidden(Path::new("/a/.env/sub")));
    }

    #[test]
    fn no_prefix_string_match() {
        let dir = tmp();
        let sub = dir.join("secrets");
        fs::create_dir_all(&sub).expect("mkdir");
        let h = set(&[sub.to_str().unwrap()]);
        assert!(!h.is_hidden(&dir.join("secrets2").join("x")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dotdot_lands_in_hidden_folder() {
        let dir = tmp();
        let sub = dir.join("secret");
        fs::create_dir_all(&sub).expect("mkdir");
        let h = set(&[sub.to_str().unwrap()]);
        let via_dotdot = dir.join("secret").join("..").join("secret").join("x");
        assert!(h.is_hidden(&via_dotdot));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn nonexistent_path_under_hidden_folder_is_hidden() {
        let dir = tmp();
        let sub = dir.join("secret");
        fs::create_dir_all(&sub).expect("mkdir");
        let h = set(&[sub.to_str().unwrap()]);
        assert!(h.is_hidden(&sub.join("newfile.txt")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_into_hidden_folder_is_hidden() {
        let dir = tmp();
        let secret = dir.join("secret");
        fs::create_dir_all(&secret).expect("mkdir");
        let target = secret.join("x.txt");
        fs::write(&target, "x").expect("write");
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        let h = set(&[secret.to_str().unwrap()]);
        assert!(h.is_hidden(&link));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_or_empty_file_hides_nothing() {
        let h = HiddenSet::empty();
        assert!(!h.is_hidden(Path::new("/a/b")));
    }

    #[test]
    fn refusal_uses_the_path_as_written() {
        let dir = tmp();
        let sub = dir.join("secret");
        fs::create_dir_all(&sub).expect("mkdir");
        let h = set(&[sub.to_str().unwrap()]);
        let shown = h.refusal(&sub.join("x.txt")).expect("refusal");
        assert!(shown.starts_with("deliberately hidden by the user: "), "got: {shown}");
        assert!(shown.ends_with("x.txt"), "got: {shown}");
        let _ = fs::remove_dir_all(&dir);
    }
}
