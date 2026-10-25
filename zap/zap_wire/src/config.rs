//! Config folder, line-list files and key=value settings, shared by zapcli
//! and zapgui. Settings writes go through a temp file and a rename, so
//! lines this module doesn't know about survive.

use std::fmt;
use std::path::{Path, PathBuf};

pub const THEME_OFF: &str = "off";
pub const DEFAULT_THEME: &str = "two-dark";

/// Theme ids and the labels shown in zap-settings. "off" turns coloring off.
pub const THEMES: &[(&str, &str)] = &[
    ("off", "Off"),
    ("two-dark", "Two Dark"),
    ("dracula", "Dracula"),
    ("nord", "Nord"),
    ("monokai", "Monokai"),
    ("solarized-dark", "Solarized Dark"),
    ("gruvbox-dark", "Gruvbox Dark"),
    ("one-half-dark", "One Half Dark"),
    ("zenburn", "Zenburn"),
];

pub fn is_known_theme(id: &str) -> bool {
    THEMES.iter().any(|(t, _)| *t == id)
}

pub fn theme_label(id: &str) -> Option<&'static str> {
    THEMES.iter().find(|(t, _)| *t == id).map(|(_, l)| *l)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    pub err_msg: String,
    pub method: &'static str,
    pub filepath: String,
}

impl ConfigError {
    fn new(err_msg: impl Into<String>, method: &'static str, path: &Path) -> Self {
        Self {
            err_msg: err_msg.into(),
            method,
            filepath: path.display().to_string(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.method, self.filepath, self.err_msg)
    }
}

impl std::error::Error for ConfigError {}

/// XDG_CONFIG_HOME/zap if that is set to an absolute path, else
/// HOME/.config/zap. A missing HOME falls back to "/", the same as
/// zapcli's home_dir.
pub fn config_dir_from(xdg: Option<&str>, home: Option<&str>) -> PathBuf {
    if let Some(x) = xdg {
        if x.starts_with('/') {
            return Path::new(x).join("zap");
        }
    }
    Path::new(home.unwrap_or("/")).join(".config").join("zap")
}

pub fn config_dir() -> PathBuf {
    // A variable that is missing or not valid unicode counts as not set.
    let xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();
    config_dir_from(xdg.as_deref(), home.as_deref())
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings")
}

fn read_to_string_or_empty(path: &Path, method: &'static str) -> Result<String, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(ConfigError::new(e.to_string(), method, path)),
    }
}

fn atomic_write(path: &Path, body: &str, method: &'static str) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ConfigError::new(e.to_string(), method, parent))?;
    }
    let mut tmp_name = path
        .file_name()
        .ok_or_else(|| ConfigError::new("path has no file name", method, path))?
        .to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    std::fs::write(&tmp, body).map_err(|e| ConfigError::new(e.to_string(), method, &tmp))?;
    std::fs::rename(&tmp, path).map_err(|e| ConfigError::new(e.to_string(), method, path))
}

/// Read a line-list file. Blank lines and lines starting with # are
/// skipped, the rest are trimmed. A missing file is an empty list.
pub fn read_lines(path: &Path) -> Result<Vec<String>, ConfigError> {
    let body = read_to_string_or_empty(path, "config::read_lines")?;
    Ok(body
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect())
}

/// Write a line-list file, one entry per line.
pub fn write_lines(path: &Path, lines: &[String]) -> Result<(), ConfigError> {
    let mut body = lines.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    atomic_write(path, &body, "config::write_lines")
}

fn line_key(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.starts_with('#') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    Some(k.trim())
}

/// First value for `key` in a key=value file. A missing file or key is None.
pub fn get_key(path: &Path, key: &str) -> Result<Option<String>, ConfigError> {
    let body = read_to_string_or_empty(path, "config::get_key")?;
    for line in body.lines() {
        if line_key(line) == Some(key) {
            if let Some((_, v)) = line.trim().split_once('=') {
                return Ok(Some(v.trim().to_string()));
            }
        }
    }
    Ok(None)
}

/// Set `key` to `value`. The first line holding the key is replaced in
/// place, a missing key is appended, a missing file is created. Every
/// other line is kept as it was.
pub fn set_key(path: &Path, key: &str, value: &str) -> Result<(), ConfigError> {
    const M: &str = "config::set_key";
    if key.is_empty() || key.contains(['=', '\n', '\r']) {
        return Err(ConfigError::new("bad key", M, path));
    }
    if value.contains(['\n', '\r']) {
        return Err(ConfigError::new("value contains a line break", M, path));
    }
    let existing = read_to_string_or_empty(path, M)?;
    let mut out: Vec<String> = Vec::new();
    let mut done = false;
    for line in existing.lines() {
        if !done && line_key(line) == Some(key) {
            out.push(format!("{key}={value}"));
            done = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !done {
        out.push(format!("{key}={value}"));
    }
    let mut body = out.join("\n");
    body.push('\n');
    atomic_write(path, &body, M)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("zap_wire_cfg_{}_{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn set_keeps_unknown_lines_and_other_keys() {
        let d = tmp("keeps");
        std::fs::create_dir_all(&d).expect("mkdir");
        let f = d.join("settings");
        std::fs::write(&f, "# note\ndefault_folder=/a\nweird line\n").expect("write");
        set_key(&f, "theme", "nord").expect("set");
        let got = std::fs::read_to_string(&f).expect("read");
        assert_eq!(got, "# note\ndefault_folder=/a\nweird line\ntheme=nord\n");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn set_replaces_existing_key_in_place() {
        let d = tmp("replace");
        std::fs::create_dir_all(&d).expect("mkdir");
        let f = d.join("settings");
        std::fs::write(&f, "a=1\ntheme=nord\nz=9\n").expect("write");
        set_key(&f, "theme", "dracula").expect("set");
        let got = std::fs::read_to_string(&f).expect("read");
        assert_eq!(got, "a=1\ntheme=dracula\nz=9\n");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn set_appends_a_missing_key() {
        let d = tmp("append");
        std::fs::create_dir_all(&d).expect("mkdir");
        let f = d.join("settings");
        std::fs::write(&f, "a=1").expect("write");
        set_key(&f, "b", "2").expect("set");
        let got = std::fs::read_to_string(&f).expect("read");
        assert_eq!(got, "a=1\nb=2\n");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn set_creates_a_missing_file_and_folder() {
        let d = tmp("create");
        let f = d.join("sub").join("settings");
        set_key(&f, "theme", "nord").expect("set");
        assert_eq!(get_key(&f, "theme").expect("get"), Some("nord".to_string()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn get_missing_file_or_key_is_none() {
        let d = tmp("get");
        let f = d.join("settings");
        assert_eq!(get_key(&f, "x").expect("get"), None);
        std::fs::create_dir_all(&d).expect("mkdir");
        std::fs::write(&f, "# x=1\ny=2\n").expect("write");
        assert_eq!(get_key(&f, "x").expect("get"), None);
        assert_eq!(get_key(&f, "y").expect("get"), Some("2".to_string()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn line_lists_skip_comments_and_blanks() {
        let d = tmp("lines");
        std::fs::create_dir_all(&d).expect("mkdir");
        let f = d.join("list");
        std::fs::write(&f, "# c\n\n  /a  \n/b\n").expect("write");
        assert_eq!(read_lines(&f).expect("read"), vec!["/a", "/b"]);
        write_lines(&f, &["/x".to_string(), "/y".to_string()]).expect("write");
        assert_eq!(std::fs::read_to_string(&f).expect("read"), "/x\n/y\n");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn config_dir_prefers_absolute_xdg() {
        assert_eq!(
            config_dir_from(Some("/x/cfg"), Some("/home/u")),
            PathBuf::from("/x/cfg/zap")
        );
        assert_eq!(
            config_dir_from(Some(""), Some("/home/u")),
            PathBuf::from("/home/u/.config/zap")
        );
        assert_eq!(
            config_dir_from(None, Some("/home/u")),
            PathBuf::from("/home/u/.config/zap")
        );
    }

    #[test]
    fn default_theme_is_in_the_list() {
        assert!(is_known_theme(DEFAULT_THEME));
        assert!(is_known_theme(THEME_OFF));
    }
}
