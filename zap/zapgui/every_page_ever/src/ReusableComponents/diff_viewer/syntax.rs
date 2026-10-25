//! Syntax tables for the diff viewer. A table holds, for every line of a
//! whole text, the colored runs of that line. It is built top to bottom
//! once, so a line in the middle of a multi-line string or comment gets
//! the right color even when the string's first line is off screen. A row
//! looks its runs up by the line number it already carries (1-based).

use std::fmt;
use std::path::Path;

use eframe::egui::Color32;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style, Theme};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

use zap_wire::config::{DEFAULT_THEME, THEME_OFF};

/// Texts over this many bytes get an empty table.
const MAX_TEXT_BYTES: usize = 400 * 1024;
/// A text with any line over this many bytes gets an empty table.
const MAX_LINE_BYTES: usize = 5_000;

#[derive(Debug)]
pub struct SyntaxError {
    pub err_msg: String,
    pub method: &'static str,
    pub filepath: String,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.method, self.filepath, self.err_msg)
    }
}

impl std::error::Error for SyntaxError {}

/// One colored stretch of a line. Byte offsets into the line text without
/// its trailing newline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub start: usize,
    pub end: usize,
    pub color: Color32,
    pub italic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxTable {
    lines: Vec<Vec<Run>>,
}

impl SyntaxTable {
    pub fn empty() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Runs of a line by its 1-based line number. Anything out of range
    /// is an empty slice.
    pub fn line(&self, lineno: u32) -> &[Run] {
        match (lineno as usize)
            .checked_sub(1)
            .and_then(|i| self.lines.get(i))
        {
            Some(runs) => runs.as_slice(),
            None => &[],
        }
    }
}

/// Tables for the two sides of one step or one surface file.
pub struct PairTables {
    pub left: SyntaxTable,
    pub right: SyntaxTable,
}

impl PairTables {
    pub fn empty() -> Self {
        Self {
            left: SyntaxTable::empty(),
            right: SyntaxTable::empty(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Toml,
}

impl Lang {
    /// rs is Rust, toml and Cargo.lock are TOML, anything else is None.
    pub fn for_path(path: &Path) -> Option<Lang> {
        let name = path.file_name()?.to_str()?;
        if name == "Cargo.lock" {
            return Some(Lang::Toml);
        }
        match path.extension()?.to_str()? {
            "rs" => Some(Lang::Rust),
            "toml" => Some(Lang::Toml),
            _ => None,
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Lang::Rust => "rs",
            Lang::Toml => "toml",
        }
    }
}

fn name_for_id(id: &str) -> Option<EmbeddedThemeName> {
    Some(match id {
        "two-dark" => EmbeddedThemeName::TwoDark,
        "dracula" => EmbeddedThemeName::Dracula,
        "nord" => EmbeddedThemeName::Nord,
        "monokai" => EmbeddedThemeName::MonokaiExtended,
        "solarized-dark" => EmbeddedThemeName::SolarizedDark,
        "gruvbox-dark" => EmbeddedThemeName::GruvboxDark,
        "one-half-dark" => EmbeddedThemeName::OneHalfDark,
        "zenburn" => EmbeddedThemeName::Zenburn,
        _ => return None,
    })
}

/// Holds the loaded syntax set and theme set. Loading is slow, so the UI
/// builds one lazily and keeps it.
pub struct SyntaxEngine {
    syntaxes: SyntaxSet,
    themes: EmbeddedLazyThemeSet,
}

impl SyntaxEngine {
    pub fn new() -> Self {
        Self {
            syntaxes: two_face::syntax::extra_newlines(),
            themes: two_face::theme::extra(),
        }
    }

    /// An id that isn't in the list uses the default theme.
    fn theme(&self, id: &str) -> &Theme {
        let name = name_for_id(id)
            .or_else(|| name_for_id(DEFAULT_THEME))
            .unwrap_or(EmbeddedThemeName::TwoDark);
        self.themes.get(name)
    }

    /// Highlight a whole text into a table. Off, an unknown extension and
    /// oversized texts give an empty table.
    pub fn table(
        &self,
        path: &Path,
        text: &str,
        theme_id: &str,
    ) -> Result<SyntaxTable, SyntaxError> {
        const M: &str = "SyntaxEngine::table";
        let fail = |msg: String| SyntaxError {
            err_msg: msg,
            method: M,
            filepath: path.display().to_string(),
        };

        if theme_id == THEME_OFF {
            return Ok(SyntaxTable::empty());
        }
        let Some(lang) = Lang::for_path(path) else {
            return Ok(SyntaxTable::empty());
        };
        if text.len() > MAX_TEXT_BYTES || text.split('\n').any(|l| l.len() > MAX_LINE_BYTES) {
            return Ok(SyntaxTable::empty());
        }
        let syntax = self
            .syntaxes
            .find_syntax_by_extension(lang.extension())
            .ok_or_else(|| fail(format!("no syntax for .{}", lang.extension())))?;

        let mut hl = HighlightLines::new(syntax, self.theme(theme_id));
        let mut lines: Vec<Vec<Run>> = Vec::new();
        for line in LinesWithEndings::from(text) {
            let content_len = line.strip_suffix('\n').unwrap_or(line).len();
            let tokens = hl
                .highlight_line(line, &self.syntaxes)
                .map_err(|e| fail(e.to_string()))?;
            lines.push(runs_from_tokens(&tokens, content_len));
        }
        Ok(SyntaxTable { lines })
    }
}

fn runs_from_tokens(tokens: &[(Style, &str)], content_len: usize) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut pos = 0usize;
    for (style, piece) in tokens {
        let start = pos;
        pos += piece.len();
        let end = pos.min(content_len);
        if start >= end {
            continue;
        }
        let color = Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);
        let italic = style.font_style.contains(FontStyle::ITALIC);
        if let Some(last) = runs.last_mut() {
            if last.end == start && last.color == color && last.italic == italic {
                last.end = end;
                continue;
            }
        }
        runs.push(Run {
            start,
            end,
            color,
            italic,
        });
    }
    runs
}

pub fn tables_for_pair(
    engine: &SyntaxEngine,
    path: &Path,
    before: &str,
    after: &str,
    theme_id: &str,
) -> Result<PairTables, SyntaxError> {
    Ok(PairTables {
        left: engine.table(path, before, theme_id)?,
        right: engine.table(path, after, theme_id)?,
    })
}

/// The theme id from the settings file, read fresh each call. A missing
/// or unknown value is the default. A read error is printed and also
/// gives the default.
pub fn configured_theme() -> String {
    match zap_wire::config::get_key(&zap_wire::config::settings_path(), "theme") {
        Ok(Some(id)) if zap_wire::config::is_known_theme(&id) => id,
        Ok(_) => DEFAULT_THEME.to_string(),
        Err(e) => {
            eprintln!("zap: {e}");
            DEFAULT_THEME.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zap_wire::config::THEMES;

    fn color_at(t: &SyntaxTable, lineno: u32, byte: usize) -> Option<Color32> {
        t.line(lineno)
            .iter()
            .find(|r| r.start <= byte && byte < r.end)
            .map(|r| r.color)
    }

    fn check_table(text: &str, t: &SyntaxTable) {
        let lines: Vec<&str> = LinesWithEndings::from(text).collect();
        assert_eq!(t.len(), lines.len());
        for (i, l) in lines.iter().enumerate() {
            let content = l.strip_suffix('\n').unwrap_or(l);
            let mut prev = 0usize;
            for r in t.line(i as u32 + 1) {
                assert!(r.start < r.end);
                assert!(r.end <= content.len());
                assert!(content.is_char_boundary(r.start));
                assert!(content.is_char_boundary(r.end));
                assert!(r.start >= prev, "runs overlap on line {}", i + 1);
                prev = r.end;
            }
        }
    }

    #[test]
    fn middle_line_of_a_multiline_string_is_string_colored() {
        let e = SyntaxEngine::new();
        let text = "let s = \"first\nmiddle\nlast\";\n";
        let t = e.table(Path::new("a.rs"), text, "two-dark").expect("table");
        let keyword = color_at(&t, 1, 0).expect("keyword run");
        let in_string = color_at(&t, 1, 9).expect("string run");
        let middle = color_at(&t, 2, 0).expect("middle run");
        assert_ne!(keyword, in_string);
        assert_eq!(middle, in_string);
        assert_eq!(t.line(2).len(), 1);
    }

    #[test]
    fn middle_line_of_a_block_comment_is_comment_colored() {
        let e = SyntaxEngine::new();
        let text = "fn x() {}\n/* one\ntwo\nthree */\n";
        let t = e.table(Path::new("a.rs"), text, "two-dark").expect("table");
        let keyword = color_at(&t, 1, 0).expect("keyword run");
        let comment = color_at(&t, 2, 4).expect("comment run");
        let middle = color_at(&t, 3, 0).expect("middle run");
        assert_ne!(keyword, comment);
        assert_eq!(middle, comment);
    }

    #[test]
    fn short_toml_has_two_distinct_colors() {
        let e = SyntaxEngine::new();
        let text = "[package]\nname = \"zap\"\nversion = 1\n";
        let t = e
            .table(Path::new("Cargo.toml"), text, "two-dark")
            .expect("table");
        let mut colors: Vec<Color32> = (1..=3)
            .flat_map(|n| t.line(n).iter().map(|r| r.color))
            .collect();
        colors.dedup();
        colors.sort_by_key(|c| c.to_array());
        colors.dedup();
        assert!(colors.len() >= 2, "got {} colors", colors.len());
    }

    #[test]
    fn cargo_lock_is_toml() {
        assert_eq!(Lang::for_path(Path::new("/x/Cargo.lock")), Some(Lang::Toml));
        assert_eq!(Lang::for_path(Path::new("/x/a.toml")), Some(Lang::Toml));
        assert_eq!(Lang::for_path(Path::new("/x/a.rs")), Some(Lang::Rust));
        let e = SyntaxEngine::new();
        let text = "# generated\n[[package]]\nname = \"a\"\n";
        let t = e
            .table(Path::new("/x/Cargo.lock"), text, "two-dark")
            .expect("table");
        assert_eq!(t.len(), 3);
        assert!(!t.line(2).is_empty());
    }

    #[test]
    fn unknown_extension_and_off_are_empty() {
        let e = SyntaxEngine::new();
        let t = e
            .table(Path::new("a.txt"), "fn a() {}\n", "two-dark")
            .expect("table");
        assert!(t.is_empty());
        let t = e
            .table(Path::new("a.rs"), "fn a() {}\n", "off")
            .expect("table");
        assert!(t.is_empty());
    }

    #[test]
    fn oversized_text_is_empty() {
        let e = SyntaxEngine::new();
        let long_line = format!("let a = \"{}\";\n", "x".repeat(6_000));
        let t = e
            .table(Path::new("a.rs"), &long_line, "two-dark")
            .expect("table");
        assert!(t.is_empty());
    }

    #[test]
    fn tables_have_one_entry_per_line_on_char_boundaries() {
        let e = SyntaxEngine::new();
        let rust = "let \u{e9} = \"\u{fc}\u{2192}\"; // \u{f1}\nfn f() {}\n\n/* a\n\u{e9} */\n";
        let t = e.table(Path::new("a.rs"), rust, "two-dark").expect("table");
        check_table(rust, &t);
        let toml = "[a]\nk = \"\u{fc}\"\n# \u{f1}\n";
        let t = e
            .table(Path::new("a.toml"), toml, "dracula")
            .expect("table");
        check_table(toml, &t);
    }

    #[test]
    fn every_listed_theme_resolves() {
        let e = SyntaxEngine::new();
        for (id, _) in THEMES {
            if *id == THEME_OFF {
                continue;
            }
            assert!(name_for_id(id).is_some(), "no theme for {id}");
            let t = e
                .table(Path::new("a.rs"), "fn a() {}\n", id)
                .expect("table");
            assert!(!t.is_empty());
        }
    }
}
