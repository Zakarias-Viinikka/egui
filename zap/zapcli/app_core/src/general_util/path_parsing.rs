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
/// Split a line into arguments, honouring a single pair of `"` or `'`
/// around any argument. Quotes are consumed, so `cat "/a b.txt"` yields
/// one argument `"/a b.txt"` (the quotes gone, the space kept). No
/// escapes; a `"` inside a `"`-quoted arg ends the quote at the next `"`.
pub fn split_quoted(input: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;
    for c in input.chars() {
        match in_quote {
            Some(q) => {
                if c == q {
                    in_quote = None;
                } else {
                    current.push(c);
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    in_quote = Some(c);
                } else if c.is_whitespace() {
                    if !current.is_empty() {
                        out.push(std::mem::take(&mut current));
                    }
                } else {
                    current.push(c);
                }
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

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