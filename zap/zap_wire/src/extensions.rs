//! An opt-in allowlist of file extensions for the writing verbs. Read
//! from extensions.txt in the config folder on every parse, so an edit
//! is never stale.

use std::path::Path;

pub const EXTENSIONS_FILENAME: &str = "extensions.txt";

/// The extensions.txt token that means "files with no dotted extension".
/// Chosen so it passes the normal token rules and cannot be confused with
/// a real extension a user is likely to have.
pub const NO_EXTENSION_TOKEN: &str = "noext";

#[derive(Clone, Debug, Default)]
pub struct ExtensionAllowlist {
    entries: Vec<String>,
    broken: bool,
}

impl ExtensionAllowlist {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_text(text: &str) -> Self {
        let mut entries: Vec<String> = Vec::new();
        let mut had_tokens = false;
        for line in text.lines() {
            let stripped = match line.split_once('#') {
                Some((before, _)) => before,
                None => line,
            };
            for tok in stripped.split(|c: char| c == ',' || c.is_whitespace()) {
                if tok.is_empty() {
                    continue;
                }
                had_tokens = true;
                let t = tok.strip_prefix('.').unwrap_or(tok);
                if t.is_empty() || t.len() > 16 {
                    continue;
                }
                if !t
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '+')
                {
                    continue;
                }
                entries.push(t.to_ascii_lowercase());
            }
        }
        let broken = had_tokens && entries.is_empty();
        Self { entries, broken }
    }

    pub fn load() -> Self {
        let path = crate::config::config_dir().join(EXTENSIONS_FILENAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::from_text(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::empty(),
            Err(_) => Self {
                entries: Vec::new(),
                broken: true,
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && !self.broken
    }

    fn allowed_list(&self) -> String {
        let shown: Vec<String> = self
            .entries
            .iter()
            .take(12)
            .map(|e| {
                if e == NO_EXTENSION_TOKEN {
                    "no extension".to_string()
                } else {
                    format!(".{e}")
                }
            })
            .collect();
        let mut s = shown.join(", ");
        if self.entries.len() > 12 {
            s.push_str(", ...");
        }
        s
    }

    pub fn check(&self, verb: &str, path: &Path) -> Option<String> {
        if !matches!(verb, "create" | "replace" | "overwrite" | "append") {
            return None;
        }
        if self.broken {
            return Some(format!(
                "{verb}: ~/.config/zap/extensions.txt has entries but none are valid extensions, fix the file or delete it to allow everything"
            ));
        }
        if self.entries.is_empty() {
            return None;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        match ext.as_deref() {
            None | Some("") => {
                if self.entries.iter().any(|x| x == NO_EXTENSION_TOKEN) {
                    None
                } else {
                    Some(format!(
                        "{}: {} has no extension, and only paths with an allowed extension (or a file with no extension, if allowed) can be written (allowed: {})",
                        verb,
                        path.display(),
                        self.allowed_list()
                    ))
                }
            }
            Some(e) => {
                if self.entries.iter().any(|x| x == e) {
                    None
                } else {
                    Some(format!(
                        "{}: .{} files are not allowed here, allowed extensions are {}",
                        verb,
                        e,
                        self.allowed_list()
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_blank_lines_leading_dots_uppercase_commas_duplicates() {
        let a = ExtensionAllowlist::from_text("# comment\n.RS, .md\n  md   .toml  \n");
        assert_eq!(a.entries, vec!["rs", "md", "md", "toml"]);
        assert!(!a.broken);
    }

    #[test]
    fn an_invalid_token_is_skipped() {
        let a = ExtensionAllowlist::from_text(".rs;.md, .md");
        assert_eq!(a.entries, vec!["md"]);
        assert!(!a.broken);
    }

    #[test]
    fn only_invalid_tokens_means_broken() {
        let a = ExtensionAllowlist::from_text(".rs;.md");
        assert!(a.entries.is_empty());
        assert!(a.broken);
    }

    #[test]
    fn empty_and_comments_only_mean_allow_all() {
        assert!(ExtensionAllowlist::from_text("").is_empty());
        assert!(ExtensionAllowlist::from_text("# nothing\n\n").is_empty());
    }

    #[test]
    fn check_allows_an_extension() {
        let a = ExtensionAllowlist::from_text("rs, md");
        assert!(a.check("create", Path::new("/tmp/a.rs")).is_none());
        assert!(a.check("create", Path::new("/tmp/A.MD")).is_none());
    }

    #[test]
    fn check_refuses_a_disallowed_extension() {
        let a = ExtensionAllowlist::from_text("rs, md");
        let msg = a.check("create", Path::new("/tmp/a.txt")).unwrap();
        assert!(msg.contains(".txt"), "got: {msg}");
        assert!(msg.contains(".rs"), "got: {msg}");
        assert!(msg.contains(".md"), "got: {msg}");
        assert!(!msg.contains(';'), "got: {msg}");
        assert!(!msg.contains('\n'), "got: {msg}");
    }

    #[test]
    fn check_refuses_an_extensionless_path() {
        let a = ExtensionAllowlist::from_text("rs");
        for p in ["/tmp/Makefile", "/tmp/.gitignore", "/tmp/a."] {
            let msg = a.check("create", Path::new(p)).unwrap();
            assert!(msg.contains("no extension"), "{p}: got {msg}");
        }
    }

    #[test]
    fn check_ignores_non_writing_verbs() {
        let a = ExtensionAllowlist::from_text("rs");
        assert!(a.check("zaprm", Path::new("/tmp/a.txt")).is_none());
        assert!(a.check("zapmv", Path::new("/tmp/a.txt")).is_none());
        assert!(a.check("mkdir", Path::new("/tmp/Makefile")).is_none());
        assert!(a.check("rmdir", Path::new("/tmp/Makefile")).is_none());
        assert!(a.check("chmod", Path::new("/tmp/Makefile")).is_none());
        assert!(a.check("run", Path::new("/tmp/x.sh")).is_none());
        assert!(a.check("cargo", Path::new("/tmp")).is_none());
    }

    #[test]
    fn broken_allowlist_refuses_checked_verbs() {
        let a = ExtensionAllowlist::from_text(".rs;.md");
        let msg = a.check("create", Path::new("/tmp/a.rs")).unwrap();
        assert!(msg.contains("fix the file"), "got: {msg}");
    }

    #[test]
    fn more_than_twelve_extensions_truncate() {
        let long: String = (0..15)
            .map(|i| format!(".e{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let a = ExtensionAllowlist::from_text(&long);
        let msg = a
            .check("create", Path::new("/tmp/a.txt"))
            .expect("refused");
        assert!(msg.contains("..."), "got: {msg}");
    }
}
