//! Chain grammar: parse a chain block into a list of steps.
//! Shared by zapcli (which parses before sending, so bad paths and
//! disallowed cargo args never reach the socket) and zapgui (which parses
//! again and enforces the same rules).
//!
//! A chain may end with one run step: a `cd <folder>` line, then a
//! `cargo <args>` or `run <file> [args]` line. The cd is a directive, not
//! a step: it sets the working folder for the run step after it.
//!
//! The parser collects every problem it can find instead of stopping at
//! the first. After an error it resumes at a point that cannot produce
//! derived errors (see `read_block`, `skip_past_end`, `next_header`).

use crate::argv;
use crate::cargo_gate;
use crate::extensions::ExtensionAllowlist;
use crate::hidden::HiddenSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Create,
    Replace,
    Overwrite,
    Append,
    Remove,
    Move,
    Mkdir,
    RemoveDir,
    Cargo,
    Run,
    Chmod,
    Rename,
    AstGrep,
}

impl Verb {
    pub const ALL: [Verb; 13] = [
        Verb::Create,
        Verb::Replace,
        Verb::Overwrite,
        Verb::Append,
        Verb::Remove,
        Verb::Move,
        Verb::Mkdir,
        Verb::RemoveDir,
        Verb::Cargo,
        Verb::Run,
        Verb::Chmod,
        Verb::Rename,
        Verb::AstGrep,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Verb::Create => "create",
            Verb::Replace => "replace",
            Verb::Overwrite => "overwrite",
            Verb::Append => "append",
            Verb::Remove => "delete",
            Verb::Move => "zapmv",
            Verb::Mkdir => "mkdir",
            Verb::RemoveDir => "rmdir",
            Verb::Cargo => "cargo",
            Verb::Run => "run",
            Verb::Chmod => "chmod",
            Verb::Rename => "rename",
            Verb::AstGrep => "astgrep",
        }
    }

    /// The verb a header word names, if any. `rm` and `mv` are not here:
    /// they are refused with a "use delete / zapmv" message.
    pub fn from_word(word: &str) -> Option<Verb> {
        Some(match word {
            "create" => Verb::Create,
            "replace" => Verb::Replace,
            "overwrite" => Verb::Overwrite,
            "append" => Verb::Append,
            "delete" => Verb::Remove,
            "zapmv" => Verb::Move,
            "mkdir" => Verb::Mkdir,
            "rmdir" => Verb::RemoveDir,
            "chmod" => Verb::Chmod,
            "rename" => Verb::Rename,
            "astgrep" => Verb::AstGrep,
            "cargo" => Verb::Cargo,
            "run" => Verb::Run,
            _ => return None,
        })
    }

    pub fn is_run(self) -> bool {
        matches!(self, Verb::Cargo | Verb::Run)
    }

    /// A step whose verb does more than write text: it runs a command,
    /// launches a file, or changes a mode. The review window gates these
    /// on Verify and Allow before Confirm will work.
    pub fn is_special(self) -> bool {
        matches!(
            self,
            Verb::Cargo | Verb::Run | Verb::Chmod | Verb::Rename | Verb::AstGrep
        )
    }

    /// A step that always goes to review, even inside a whitelisted folder.
    pub fn always_reviewed(self) -> bool {
        matches!(
            self,
            Verb::Remove
                | Verb::Move
                | Verb::RemoveDir
                | Verb::Chmod
                | Verb::Cargo
                | Verb::Run
                | Verb::Rename
                | Verb::AstGrep
        )
    }

    pub fn has_editable_target(self) -> bool {
        matches!(
            self,
            Verb::Create | Verb::Mkdir | Verb::Move | Verb::Rename
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub verb: Verb,
    /// Absolute path or `~/...`. For Cargo and Run, the resolved folder
    /// from the preceding `cd`.
    pub path: String,
    /// Move only: the destination path.
    pub dest: Option<String>,
    /// replace only: the text to find.
    pub old_text: String,
    /// create, overwrite, replace, append: the text to put in place.
    pub new_text: String,
    /// replace only: how many occurrences to replace. None means one.
    pub count: Option<usize>,
    /// create and append only: don't add a trailing newline.
    pub no_newline: bool,
    /// Cargo: the args after "cargo". Run: [file, extra args...].
    pub args: Vec<String>,
    /// Cargo and Run only: the folder from the preceding cd.
    pub cwd: Option<String>,
}

impl Step {
    /// Assign a new target for the edit_title pen. The caller validates
    /// the text before calling; this only sets what it can.
    pub fn set_target(&mut self, target: &str) -> Result<(), String> {
        match self.verb {
            Verb::Create | Verb::Mkdir => {
                self.path = target.to_string();
                Ok(())
            }
            Verb::Move => {
                self.dest = Some(target.to_string());
                Ok(())
            }
            Verb::Rename => {
                let parent = std::path::Path::new(&self.path)
                    .parent()
                    .ok_or_else(|| "rename: no parent folder".to_string())?;
                let dest = parent.join(target);
                self.dest = Some(dest.to_string_lossy().into_owned());
                Ok(())
            }
            _ => Err(format!(
                "{} steps have no editable target",
                self.verb.name()
            )),
        }
    }
}

/// One problem found in a chain. Displays as one clause:
/// `<message> at line <n>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at line {}", self.message, self.line)
    }
}

impl std::error::Error for ParseError {}

/// At most this many clauses are printed; the rest are counted.
const MAX_CLAUSES: usize = 20;

/// Every problem found in one chain: non-empty, sorted by line (stable),
/// exact duplicates dropped. Displays as one sentence, clauses joined by
/// "; ", with " (+ N more)" after the twentieth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseFailure {
    errors: Vec<ParseError>,
}

impl ParseFailure {
    /// None when the list is empty, so a ParseFailure is never empty.
    pub fn from_errors(mut errors: Vec<ParseError>) -> Option<ParseFailure> {
        errors.sort_by_key(|e| e.line);
        let mut out: Vec<ParseError> = Vec::new();
        for e in errors {
            if !out.contains(&e) {
                out.push(e);
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(ParseFailure { errors: out })
        }
    }

    /// The error on the earliest line.
    pub fn first(&self) -> &ParseError {
        &self.errors[0]
    }

    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}

impl std::fmt::Display for ParseFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shown: Vec<String> = self
            .errors
            .iter()
            .take(MAX_CLAUSES)
            .map(|e| e.to_string())
            .collect();
        write!(f, "{}", shown.join("; "))?;
        if self.errors.len() > MAX_CLAUSES {
            write!(f, " (+ {} more)", self.errors.len() - MAX_CLAUSES)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseFailure {}

/// Inputs the chain parser needs that are not part of the chain text:
/// the hidden set and the extension allowlist.
pub struct ParseConfig {
    pub hidden: HiddenSet,
    pub extensions: ExtensionAllowlist,
}

impl ParseConfig {
    /// No hidden paths, no allowlist. Tests use this so they never read
    /// the user's files.
    pub fn empty() -> Self {
        Self {
            hidden: HiddenSet::empty(),
            extensions: ExtensionAllowlist::empty(),
        }
    }

    /// The real config: read hidden.txt and extensions.txt from the
    /// config folder.
    pub fn load() -> Self {
        Self {
            hidden: HiddenSet::load(),
            extensions: ExtensionAllowlist::load(),
        }
    }
}

/// Parse with the real config. Every caller goes through this, so nothing
/// can skip the hidden-path rule.
pub fn parse(input: &str) -> Result<Vec<Step>, ParseFailure> {
    let config = ParseConfig::load();
    parse_with(input, &config)
}

fn stray_cd(line: usize) -> ParseError {
    ParseError {
        line,
        message: "cd must be followed by cargo or run".to_string(),
    }
}

/// After a run step, the first later header or cd is reported once.
fn note_after_run(
    errors: &mut Vec<ParseError>,
    saw_run: bool,
    complained: &mut bool,
    line: usize,
) {
    if saw_run && !*complained {
        *complained = true;
        errors.push(ParseError {
            line,
            message: "a run step must be the last step".to_string(),
        });
    }
}

pub fn parse_with(input: &str, config: &ParseConfig) -> Result<Vec<Step>, ParseFailure> {
    let lines: Vec<&str> = input.lines().collect();
    let mut steps: Vec<Step> = Vec::new();
    let mut errors: Vec<ParseError> = Vec::new();
    let mut i = 0;
    // (folder, cd line, already reported). A cd that failed its own checks
    // is still a cd, so the run step under it does not complain again.
    let mut pending_cwd: Option<(String, usize, bool)> = None;
    let mut saw_run = false;
    let mut run_complained = false;
    let mut astgrep_seen = 0usize;

    while i < lines.len() {
        if lines[i].trim_end().is_empty() {
            i += 1;
            continue;
        }
        let header_no = i + 1;
        let line = lines[i].trim_end();

        // cd is a directive for a following run step, not a step itself.
        if let Some(folder) = try_cd(line) {
            note_after_run(&mut errors, saw_run, &mut run_complained, header_no);
            if matches!(&pending_cwd, Some((_, _, false))) {
                errors.push(stray_cd(header_no));
            }
            let mut reported = false;
            if folder.contains("&&") || folder.contains(';') || folder.contains('|') {
                errors.push(ParseError {
                    line: header_no,
                    message: "cd: &&, ; and | are not supported; put cd on its own line and the cargo or run line under it".to_string(),
                });
                reported = true;
            } else if !is_absolute_or_home(&folder) {
                errors.push(ParseError {
                    line: header_no,
                    message: format!(
                        "cd: folder must be absolute or start with ~/ (got {folder})"
                    ),
                });
                reported = true;
            } else if let Some(msg) = config.hidden.refusal(std::path::Path::new(&folder)) {
                errors.push(ParseError { line: header_no, message: msg });
                reported = true;
            }
            pending_cwd = Some((folder.to_string(), header_no, reported));
            i += 1;
            continue;
        }

        let (verb, rest) = match parse_header(line, header_no) {
            Ok(v) => v,
            Err(e) => {
                errors.push(e);
                i += 1;
                // The header is unusable. If a body follows, skip it whole
                // so its marker lines are not reported as unknown commands.
                if i < lines.len() && is_marker_line(lines[i]) {
                    i = skip_past_end(&lines, i);
                }
                continue;
            }
        };
        i += 1;

        let after_run = saw_run;
        note_after_run(&mut errors, saw_run, &mut run_complained, header_no);

        match verb {
            Verb::Remove
            | Verb::Move
            | Verb::Mkdir
            | Verb::RemoveDir
            | Verb::Chmod
            | Verb::Rename
            | Verb::AstGrep => {
                if let Some((_, cd_line, reported)) = pending_cwd.take() {
                    if !reported {
                        errors.push(stray_cd(cd_line));
                    }
                }
                match parse_single_line(verb, rest, header_no, config) {
                    Ok(new_steps) => {
                        if verb == Verb::AstGrep {
                            astgrep_seen += 1;
                            if astgrep_seen > 1 {
                                errors.push(ParseError {
                                    line: header_no,
                                    message: "astgrep: only one astgrep step per chain"
                                        .to_string(),
                                });
                            }
                        }
                        if !after_run {
                            steps.extend(new_steps);
                        }
                    }
                    Err(e) => errors.push(e),
                }
            }
            Verb::Create | Verb::Replace | Verb::Overwrite | Verb::Append => {
                if let Some((_, cd_line, reported)) = pending_cwd.take() {
                    if !reported {
                        errors.push(stray_cd(cd_line));
                    }
                }
                let (old_text, new_text, next_i) =
                    match read_block(&lines, i, verb, header_no) {
                        Ok(v) => v,
                        Err((e, resume)) => {
                            errors.push(e);
                            i = resume;
                            continue;
                        }
                    };
                // The block is consumed. Problems with the header line
                // itself resume right after it.
                i = next_i;

                let (path, count, no_newline) =
                    match parse_path_and_flags(rest, header_no, verb, config) {
                        Ok(v) => v,
                        Err(e) => {
                            errors.push(e);
                            continue;
                        }
                    };

                if let Some(msg) =
                    config.extensions.check(verb.name(), std::path::Path::new(&path))
                {
                    errors.push(ParseError { line: header_no, message: msg });
                    continue;
                }

                let new_text = if verb == Verb::Create && !no_newline {
                    let mut t = new_text;
                    if !t.ends_with('\n') {
                        t.push('\n');
                    }
                    t
                } else {
                    new_text
                };

                if !after_run {
                    steps.push(Step {
                        verb,
                        path,
                        dest: None,
                        old_text,
                        new_text,
                        count,
                        no_newline,
                        args: Vec::new(),
                        cwd: None,
                    });
                }
            }
            Verb::Cargo | Verb::Run => {
                let Some((folder, _cd_line, _reported)) = pending_cwd.take() else {
                    errors.push(ParseError {
                        line: header_no,
                        message: format!(
                            "{}: needs a preceding 'cd <folder>' line",
                            verb.name()
                        ),
                    });
                    continue;
                };
                // From here this line is the run step, whatever its
                // arguments look like.
                saw_run = true;
                let mut ok = true;
                let args = match argv::split(rest.trim()) {
                    Ok(a) => a,
                    Err(e) => {
                        errors.push(ParseError {
                            line: header_no,
                            message: format!("{}: {}", verb.name(), e),
                        });
                        ok = false;
                        Vec::new()
                    }
                };
                if ok && args.is_empty() {
                    errors.push(ParseError {
                        line: header_no,
                        message: format!("{}: missing arguments", verb.name()),
                    });
                    ok = false;
                }
                if ok {
                    if verb == Verb::Cargo {
                        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                        if let Err(e) = cargo_gate::check(&refs) {
                            errors.push(ParseError { line: header_no, message: e });
                            ok = false;
                        }
                    } else if !is_absolute_or_home(&args[0]) {
                        errors.push(ParseError {
                            line: header_no,
                            message: format!(
                                "run: file must be absolute or start with ~/ (got {})",
                                args[0]
                            ),
                        });
                        ok = false;
                    } else if let Some(msg) =
                        config.hidden.refusal(std::path::Path::new(&args[0]))
                    {
                        errors.push(ParseError { line: header_no, message: msg });
                        ok = false;
                    }
                }
                if ok && !after_run {
                    steps.push(Step {
                        verb,
                        path: folder.clone(),
                        dest: None,
                        old_text: String::new(),
                        new_text: String::new(),
                        count: None,
                        no_newline: false,
                        args,
                        cwd: Some(folder),
                    });
                }
            }
        }
    }

    if let Some((_, cd_line, false)) = pending_cwd {
        errors.push(stray_cd(cd_line));
    }

    match ParseFailure::from_errors(errors) {
        Some(failure) => Err(failure),
        None => Ok(steps),
    }
}

fn try_cd(line: &str) -> Option<String> {
    let (word, rest) = match line.find(char::is_whitespace) {
        Some(idx) => (&line[..idx], line[idx + 1..].trim()),
        None => (line, ""),
    };
    if word == "cd" && !rest.is_empty() {
        Some(strip_one_quote_pair(rest))
    } else {
        None
    }
}

/// Strip one matching pair of `"` or `'` from around `s`, if present.
/// The whole string must be wrapped in the same quote char; otherwise
/// nothing changes.
fn strip_one_quote_pair(s: &str) -> String {
    let bytes = s.as_bytes();
    if s.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' || first == b'\'') && first == last {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// A line that starts a new step: its first word is a verb, or cd, rm or mv.
fn header_looking(line: &str) -> bool {
    let word = line.split(char::is_whitespace).next().unwrap_or("");
    Verb::from_word(word).is_some() || matches!(word, "cd" | "rm" | "mv")
}

/// Index just after the first `@end` line at or after `from`, or the end
/// of input when there is none.
fn skip_past_end(lines: &[&str], from: usize) -> usize {
    lines
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, l)| l.trim_end() == "@end")
        .map(|(j, _)| j + 1)
        .unwrap_or(lines.len())
}

/// Index of the first header-looking line at or after `from`, or the end
/// of input when there is none.
fn next_header(lines: &[&str], from: usize) -> usize {
    lines
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, l)| header_looking(l))
        .map(|(j, _)| j)
        .unwrap_or(lines.len())
}

fn parse_header(line: &str, line_no: usize) -> Result<(Verb, &str), ParseError> {
    let (word, rest) = match line.find(char::is_whitespace) {
        Some(idx) => (&line[..idx], &line[idx + 1..]),
        None => (line, ""),
    };
    let Some(verb) = Verb::from_word(word) else {
        let message = match word {
            "rm" => "use delete".to_string(),
            "mv" => "use zapmv".to_string(),
            _ => format!("unknown command: {:?}", word),
        };
        return Err(ParseError { line: line_no, message });
    };
    if rest.trim().is_empty() {
        return Err(ParseError {
            line: line_no,
            message: format!("{}: missing path", verb.name()),
        });
    }
    Ok((verb, rest))
}

/// Read the marker structure of a create, replace, overwrite or append
/// that starts at line index `i` (the line after the header). Returns the
/// old text, the new text and the index after the block. On failure
/// returns the error and the index to resume at.
fn read_block(
    lines: &[&str],
    i: usize,
    verb: Verb,
    header_no: usize,
) -> Result<(String, String, usize), (ParseError, usize)> {
    if i >= lines.len() {
        return Err((
            ParseError {
                line: header_no,
                message: format!("{}: expected @new", verb.name()),
            },
            lines.len(),
        ));
    }
    if !is_marker_line(lines[i]) {
        // A body was forgotten, or a marker was misspelled. If the line
        // starts with `@`, report what was found so the repair module can
        // suggest the closest real marker. Otherwise just say what was
        // expected.
        let t = lines[i].trim_end();
        let expected_first = if verb == Verb::Replace { "old" } else { "new" };
        let message = if t.starts_with('@') {
            format!("{}: expected @{}, got {}", verb.name(), expected_first, t)
        } else {
            format!("{}: expected @{}", verb.name(), expected_first)
        };
        return Err((
            ParseError { line: i + 1, message },
            next_header(lines, i),
        ));
    }
    let first_name = marker_name(lines[i]).to_string();
    if verb == Verb::Replace {
        if first_name != "old" {
            return Err((
                ParseError {
                    line: i + 1,
                    message: format!("replace: expected @old, got @{}", first_name),
                },
                skip_past_end(lines, i),
            ));
        }
    } else if first_name != "new" {
        return Err((
            ParseError {
                line: i + 1,
                message: format!("{}: expected @new, got @{}", verb.name(), first_name),
            },
            skip_past_end(lines, i),
        ));
    }

    let body = |from: usize, expected: &str| {
        read_body(lines, from, expected).map_err(|e| {
            let resume = skip_past_end(lines, e.line.saturating_sub(1));
            (e, resume)
        })
    };

    if verb == Verb::Replace {
        let (old_text, after_old) = body(i + 1, "new")?;
        let (new_text, after_new) = body(after_old, "end")?;
        Ok((old_text, new_text, after_new))
    } else {
        let (new_text, after_new) = body(i + 1, "end")?;
        Ok((String::new(), new_text, after_new))
    }
}

fn parse_single_line(
    verb: Verb,
    rest: &str,
    line: usize,
    config: &ParseConfig,
) -> Result<Vec<Step>, ParseError> {
    let args = argv::split(rest.trim()).map_err(|e| ParseError {
        line,
        message: format!("{}: {}", verb.name(), e),
    })?;
    if args.is_empty() {
        return Err(ParseError {
            line,
            message: format!("{}: missing path", verb.name()),
        });
    }

    match verb {
        Verb::Remove => {
            let mut steps = Vec::new();
            for a in &args {
                if a.starts_with('-') && a.len() > 1 {
                    return Err(ParseError {
                        line,
                        message: format!("delete: unsupported option {a}"),
                    });
                }
                require_absolute(a, line, verb, config)?;
                steps.push(Step {
                    verb: Verb::Remove,
                    path: a.clone(),
                    dest: None,
                    old_text: String::new(),
                    new_text: String::new(),
                    count: None,
                    no_newline: false,
                    args: Vec::new(),
                    cwd: None,
                });
            }
            Ok(steps)
        }
        Verb::Move => {
            if args.len() != 2 {
                return Err(ParseError {
                    line,
                    message: "mv: needs a source and a destination".to_string(),
                });
            }
            if args.iter().any(|a| a.starts_with('-') && a.len() > 1) {
                return Err(ParseError {
                    line,
                    message: "mv: flags are not supported".to_string(),
                });
            }
            require_absolute(&args[0], line, verb, config)?;
            require_absolute(&args[1], line, verb, config)?;
            Ok(vec![Step {
                verb: Verb::Move,
                path: args[0].clone(),
                dest: Some(args[1].clone()),
                old_text: String::new(),
                new_text: String::new(),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            }])
        }
        Verb::Mkdir => {
            let mut steps = Vec::new();
            for a in &args {
                if a == "-p" {
                    continue;
                }
                if a.starts_with('-') && a.len() > 1 {
                    return Err(ParseError {
                        line,
                        message: format!("mkdir: unsupported option {a}"),
                    });
                }
                require_absolute(a, line, verb, config)?;
                steps.push(Step {
                    verb: Verb::Mkdir,
                    path: a.clone(),
                    dest: None,
                    old_text: String::new(),
                    new_text: String::new(),
                    count: None,
                    no_newline: false,
                    args: Vec::new(),
                    cwd: None,
                });
            }
            if steps.is_empty() {
                return Err(ParseError {
                    line,
                    message: "mkdir: missing path".to_string(),
                });
            }
            Ok(steps)
        }
        Verb::RemoveDir => {
            let mut steps = Vec::new();
            for a in &args {
                if a.starts_with('-') && a.len() > 1 {
                    return Err(ParseError {
                        line,
                        message: format!("rmdir: unsupported option {a}"),
                    });
                }
                require_absolute(a, line, verb, config)?;
                steps.push(Step {
                    verb: Verb::RemoveDir,
                    path: a.clone(),
                    dest: None,
                    old_text: String::new(),
                    new_text: String::new(),
                    count: None,
                    no_newline: false,
                    args: Vec::new(),
                    cwd: None,
                });
            }
            if steps.is_empty() {
                return Err(ParseError {
                    line,
                    message: "rmdir: missing path".to_string(),
                });
            }
            Ok(steps)
        }
        Verb::Chmod => {
            if args[0] != "+x" {
                return Err(ParseError {
                    line,
                    message: format!("chmod: only +x is supported (got {})", args[0]),
                });
            }
            if args.len() < 2 {
                return Err(ParseError {
                    line,
                    message: "chmod: missing path".to_string(),
                });
            }
            let mut steps = Vec::new();
            for a in &args[1..] {
                require_absolute(a, line, Verb::Chmod, config)?;
                steps.push(Step {
                    verb: Verb::Chmod,
                    path: a.clone(),
                    dest: None,
                    old_text: String::new(),
                    new_text: String::new(),
                    count: None,
                    no_newline: false,
                    args: Vec::new(),
                    cwd: None,
                });
            }
            Ok(steps)
        }
        Verb::Rename => {
            if args.iter().any(|a| a.starts_with('-') && a.len() > 1) {
                return Err(ParseError {
                    line,
                    message: "rename: flags are not supported".to_string(),
                });
            }
            if args.len() != 2 {
                return Err(ParseError {
                    line,
                    message: "rename: needs a path and a new name".to_string(),
                });
            }
            let old = &args[0];
            let new_name = &args[1];
            if old == "~" || old.ends_with('/') {
                return Err(ParseError {
                    line,
                    message: "rename: path must name a file".to_string(),
                });
            }
            require_absolute(old, line, Verb::Rename, config)?;
            if new_name.is_empty() {
                return Err(ParseError {
                    line,
                    message: "rename: the new name is empty".to_string(),
                });
            }
            if new_name.contains('/') {
                return Err(ParseError {
                    line,
                    message: "rename: the new name must be a bare file name, use zapmv to change folders".to_string(),
                });
            }
            if new_name == "." || new_name == ".." {
                return Err(ParseError {
                    line,
                    message: "rename: the new name must be a bare file name".to_string(),
                });
            }
            let old_name = std::path::Path::new(old)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .ok_or_else(|| ParseError {
                    line,
                    message: "rename: path must name a file".to_string(),
                })?;
            if new_name == &old_name {
                return Err(ParseError {
                    line,
                    message: "rename: the new name is the same as the old one".to_string(),
                });
            }
            let parent = std::path::Path::new(old).parent().ok_or_else(|| ParseError {
                line,
                message: "rename: path must name a file".to_string(),
            })?;
            let dest_path = parent.join(new_name);
            if let Some(msg) = config.hidden.refusal(&dest_path) {
                return Err(ParseError { line, message: msg });
            }
            Ok(vec![Step {
                verb: Verb::Rename,
                path: old.clone(),
                dest: Some(dest_path.to_string_lossy().into_owned()),
                old_text: String::new(),
                new_text: String::new(),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            }])
        }
        Verb::AstGrep => {
            if args[0] != "rename" {
                return Err(ParseError {
                    line,
                    message: format!("astgrep: only rename is supported (got {})", args[0]),
                });
            }
            if args.len() != 4 {
                return Err(ParseError {
                    line,
                    message: "astgrep: needs rename, a folder, an old name and a new name"
                        .to_string(),
                });
            }
            require_absolute(&args[1], line, verb, config)?;
            let old = &args[2];
            let new_name = &args[3];
            for (which, name) in [("old", old), ("new", new_name)] {
                if let Some(reason) = identifier_problem(name) {
                    return Err(ParseError {
                        line,
                        message: format!("astgrep: the {which} name `{name}` {reason}"),
                    });
                }
            }
            if old == new_name {
                return Err(ParseError {
                    line,
                    message: "astgrep: the new name is the same as the old one".to_string(),
                });
            }
            Ok(vec![Step {
                verb: Verb::AstGrep,
                path: args[1].clone(),
                dest: None,
                old_text: old.clone(),
                new_text: new_name.clone(),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            }])
        }
        _ => unreachable!(),
    }
}

fn require_absolute(
    path: &str,
    line: usize,
    verb: Verb,
    config: &ParseConfig,
) -> Result<(), ParseError> {
    if !is_absolute_or_home(path) {
        return Err(ParseError {
            line,
            message: format!(
                "{}: path must be absolute or start with ~/ (got {})",
                verb.name(),
                path
            ),
        });
    }
    if let Some(msg) = config.hidden.refusal(std::path::Path::new(path)) {
        return Err(ParseError { line, message: msg });
    }
    Ok(())
}

/// Words an astgrep rename may not use as an old or new name: the strict,
/// reserved and 2018+ keywords of Rust.
pub const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen",
    "if", "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override",
    "priv", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
    "true", "try", "type", "typeof", "unsafe", "unsized", "use", "virtual", "where",
    "while", "yield",
];

/// Why a name is not an acceptable plain ASCII identifier, as a phrase
/// that follows the name. None when it is acceptable.
pub fn identifier_problem(name: &str) -> Option<&'static str> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Some("is empty");
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Some("must start with a letter or underscore");
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some("may only use ASCII letters, digits and underscore");
    }
    if name == "_" {
        return Some("can't be a lone underscore");
    }
    if RUST_KEYWORDS.contains(&name) {
        return Some("is a Rust keyword");
    }
    None
}

fn is_absolute_or_home(p: &str) -> bool {
    p == "~" || p.starts_with("~/") || p.starts_with('/')
}

fn parse_path_and_flags(
    rest: &str,
    line: usize,
    verb: Verb,
    config: &ParseConfig,
) -> Result<(String, Option<usize>, bool), ParseError> {
    let mut rest = rest.trim().to_string();
    if rest.is_empty() {
        return Err(ParseError {
            line,
            message: format!("{}: missing path", verb.name()),
        });
    }

    let mut count: Option<usize> = None;
    let mut no_newline = false;

    if verb == Verb::Replace {
        if let Some(idx) = rest.rfind(char::is_whitespace) {
            let tail = &rest[idx + 1..];
            if let Some(digits) = tail.strip_prefix('x') {
                if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                    count = Some(digits.parse().map_err(|_| ParseError {
                        line,
                        message: format!("replace: bad count: {tail}"),
                    })?);
                    rest = rest[..idx].trim_end().to_string();
                }
            }
        }
    } else if matches!(verb, Verb::Create | Verb::Append) {
        if rest.ends_with(" --no-newline") {
            no_newline = true;
            rest = rest[..rest.len() - " --no-newline".len()]
                .trim_end()
                .to_string();
        }
    }

    rest = strip_one_quote_pair(&rest);

    if rest.is_empty() {
        return Err(ParseError {
            line,
            message: format!("{}: missing path", verb.name()),
        });
    }

    if !is_absolute_or_home(&rest) {
        return Err(ParseError {
            line,
            message: format!(
                "{}: path must be absolute or start with ~/ (got {})",
                verb.name(),
                rest
            ),
        });
    }
    if let Some(msg) = config.hidden.refusal(std::path::Path::new(&rest)) {
        return Err(ParseError { line, message: msg });
    }

    Ok((rest, count, no_newline))
}

fn is_marker_line(line: &str) -> bool {
    let t = line.trim_end();
    matches!(t, "@new" | "@old" | "@end")
}

fn marker_name(line: &str) -> &str {
    let t = line.trim_end();
    t.get(1..).unwrap_or("")
}

fn read_body(
    lines: &[&str],
    start: usize,
    expected: &str,
) -> Result<(String, usize), ParseError> {
    let mut text = String::new();
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim_end();
        if t == format!("@{}", expected) {
            if text.ends_with('\n') {
                text.pop();
            }
            return Ok((text, i + 1));
        }
        if is_marker_line(lines[i]) {
            return Err(ParseError {
                line: i + 1,
                message: format!("expected @{}, got {}", expected, t),
            });
        }
        text.push_str(unescape_marker_line(lines[i]));
        text.push('\n');
        i += 1;
    }
    Err(ParseError {
        line: lines.len() + 1,
        message: format!("missing @{}", expected),
    })
}

fn unescape_marker_line(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut ats = 0;
    while ats < bytes.len() && bytes[ats] == b'@' {
        ats += 1;
    }
    if ats < 2 {
        return line;
    }
    let after = line[ats..].trim_end();
    if matches!(after, "new" | "old" | "end") {
        return &line[1..];
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shadow the crate's `parse` so tests never read the user's files.
    /// Returns the first error so single-error tests read as before.
    fn parse(input: &str) -> Result<Vec<Step>, ParseError> {
        parse_with(input, &ParseConfig::empty()).map_err(|f| f.first().clone())
    }

    /// The whole failure, for tests about collected errors.
    fn parse_all(input: &str) -> Result<Vec<Step>, ParseFailure> {
        parse_with(input, &ParseConfig::empty())
    }

    fn hidden_config(entries: &[&str]) -> ParseConfig {
        ParseConfig {
            hidden: HiddenSet::from_lines(
                &entries.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            ),
            extensions: ExtensionAllowlist::empty(),
        }
    }

    fn allowlist_config(spec: &str) -> ParseConfig {
        ParseConfig {
            hidden: HiddenSet::empty(),
            extensions: ExtensionAllowlist::from_text(spec),
        }
    }

    fn parse_hidden(input: &str, entries: &[&str]) -> Result<Vec<Step>, ParseError> {
        parse_with(input, &hidden_config(entries)).map_err(|f| f.first().clone())
    }

    fn parse_cfg(input: &str, config: &ParseConfig) -> Result<Vec<Step>, ParseError> {
        parse_with(input, config).map_err(|f| f.first().clone())
    }

    #[test]
    fn parses_a_create() {
        let steps =
            parse("create /tmp/a.rs\n@new\nfn main() {}\n@end\n").expect("parse");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].verb, Verb::Create);
        assert_eq!(steps[0].path, "/tmp/a.rs");
        assert_eq!(steps[0].new_text, "fn main() {}\n");
        assert_eq!(steps[0].no_newline, false);
    }

    #[test]
    fn parses_a_replace() {
        let steps =
            parse("replace /tmp/a.rs\n@old\nfoo\n@new\nbar\n@end\n").expect("parse");
        assert_eq!(steps[0].verb, Verb::Replace);
        assert_eq!(steps[0].old_text, "foo");
        assert_eq!(steps[0].new_text, "bar");
    }

    #[test]
    fn parses_a_replace_with_count() {
        let steps = parse("replace /tmp/a.rs x2\n@old\nfoo\n@new\nbar\n@end\n")
            .expect("parse");
        assert_eq!(steps[0].count, Some(2));
    }

    #[test]
    fn parses_an_append() {
        let steps =
            parse("append /tmp/a.rs\n@new\nmore\n@end\n").expect("parse");
        assert_eq!(steps[0].verb, Verb::Append);
        assert_eq!(steps[0].new_text, "more");
    }

    #[test]
    fn create_honours_no_newline_flag() {
        let steps =
            parse("create /tmp/a.rs --no-newline\n@new\nno tail\n@end\n").expect("parse");
        assert_eq!(steps[0].new_text, "no tail");
        assert!(steps[0].no_newline);
    }

    #[test]
    fn escape_double_at_end_becomes_literal() {
        let input = "create /tmp/a.rs\n@new\nline1\n@@end\nline2\n@end\n";
        let steps = parse(input).expect("parse");
        assert_eq!(steps[0].new_text, "line1\n@end\nline2\n");
    }

    #[test]
    fn escape_triple_at_end_becomes_double() {
        let input = "create /tmp/a.rs\n@new\n@@@end\n@end\n";
        let steps = parse(input).expect("parse");
        assert_eq!(steps[0].new_text, "@@end\n");
    }

    #[test]
    fn patch_hunk_header_survives() {
        let input =
            "create /tmp/a.rs\n@new\ndiff --git a/f b/f\n@@ -1,3 +1,4 @@\n line\n@end\n";
        let steps = parse(input).expect("parse");
        assert_eq!(
            steps[0].new_text,
            "diff --git a/f b/f\n@@ -1,3 +1,4 @@\n line\n"
        );
    }

    #[test]
    fn attribute_line_survives() {
        let input = "create /tmp/a.rs\n@new\n@derive(Debug)\nstruct S;\n@end\n";
        let steps = parse(input).expect("parse");
        assert_eq!(steps[0].new_text, "@derive(Debug)\nstruct S;\n");
    }

    #[test]
    fn old_extra_at_marker_form_is_rejected() {
        let err =
            parse("create /tmp/a.rs\n@@new\nfoo\n@@end\n").expect_err("should fail");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("@new"), "got: {}", err.message);
    }

    #[test]
    fn refuses_relative_create_path() {
        let err =
            parse("create src/a.rs\n@new\nx\n@end\n").expect_err("should fail");
        assert!(err.message.contains("absolute"), "got: {}", err.message);
    }

    #[test]
    fn parses_a_remove() {
        let steps = parse("delete /tmp/a.rs\n").expect("parse");
        assert_eq!(steps[0].verb, Verb::Remove);
    }

    #[test]
    fn parses_a_move() {
        let steps = parse("zapmv /tmp/a.rs /tmp/b.rs\n").expect("parse");
        assert_eq!(steps[0].dest.as_deref(), Some("/tmp/b.rs"));
    }

    #[test]
    fn old_rm_name_is_rejected() {
        let err = parse("rm /tmp/a.rs\n").expect_err("should fail");
        assert!(err.message.contains("delete"), "got: {}", err.message);
    }

    #[test]
    fn old_mv_name_is_rejected() {
        let err = parse("mv /tmp/a.rs /tmp/b.rs\n").expect_err("should fail");
        assert!(err.message.contains("zapmv"), "got: {}", err.message);
    }

    #[test]
    fn parses_a_mkdir() {
        let steps = parse("mkdir /tmp/newdir\n").expect("parse");
        assert_eq!(steps[0].verb, Verb::Mkdir);
    }

    #[test]
    fn parses_mkdir_with_p_flag() {
        let steps = parse("mkdir -p /tmp/newdir\n").expect("parse");
        assert_eq!(steps[0].path, "/tmp/newdir");
    }

    #[test]
    fn parses_rmdir() {
        let steps = parse("rmdir /tmp/olddir\n").expect("parse");
        assert_eq!(steps[0].verb, Verb::RemoveDir);
    }

    #[test]
    fn rmdir_rejects_options() {
        let err = parse("rmdir -p /tmp/olddir\n").expect_err("should fail");
        assert!(
            err.message.contains("unsupported option"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn rmdir_needs_absolute_path() {
        let err = parse("rmdir olddir\n").expect_err("should fail");
        assert!(err.message.contains("absolute"), "got: {}", err.message);
    }

    #[test]
    fn parses_cd_then_cargo() {
        let steps = parse("cd /tmp/proj\ncargo test --release\n").expect("parse");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].verb, Verb::Cargo);
        assert_eq!(steps[0].args, vec!["test", "--release"]);
        assert_eq!(steps[0].cwd.as_deref(), Some("/tmp/proj"));
    }

    #[test]
    fn parses_cd_then_run() {
        let steps = parse("cd /tmp\nrun /tmp/x.sh arg1 arg2\n").expect("parse");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].verb, Verb::Run);
        assert_eq!(steps[0].args, vec!["/tmp/x.sh", "arg1", "arg2"]);
    }

    #[test]
    fn cargo_without_cd_is_rejected() {
        let err = parse("cargo test\n").expect_err("should fail");
        assert!(err.message.contains("cd"), "got: {}", err.message);
    }

    #[test]
    fn run_without_cd_is_rejected() {
        let err = parse("run /tmp/x.sh\n").expect_err("should fail");
        assert!(err.message.contains("cd"), "got: {}", err.message);
    }

    #[test]
    fn two_run_steps_are_rejected() {
        let err = parse("cd /tmp\ncargo test\ncd /tmp\ncargo build\n")
            .expect_err("should fail");
        assert!(
            err.message.contains("last step"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn step_after_run_is_rejected() {
        let err = parse(
            "cd /tmp\ncargo test\nreplace /tmp/a.rs\n@old\nx\n@new\ny\n@end\n",
        )
        .expect_err("should fail");
        assert!(err.message.contains("last step"), "got: {}", err.message);
    }

    #[test]
    fn run_file_must_be_absolute() {
        let err = parse("cd /tmp\nrun ./x.sh\n").expect_err("should fail");
        assert!(err.message.contains("absolute"), "got: {}", err.message);
    }

    #[test]
    fn cargo_install_is_rejected_by_gate() {
        let err = parse("cd /tmp\ncargo install cargo-edit\n")
            .expect_err("should fail");
        assert!(err.message.contains("install"), "got: {}", err.message);
    }

    #[test]
    fn cd_without_following_run_is_rejected() {
        let err = parse("cd /tmp\n").expect_err("should fail");
        assert!(err.message.contains("cd must be followed"), "got: {}", err.message);
    }

    #[test]
    fn cd_before_normal_step_is_rejected() {
        let err = parse("cd /tmp\ncreate /tmp/a.rs\n@new\nx\n@end\n")
            .expect_err("should fail");
        assert!(err.message.contains("cd must be followed"), "got: {}", err.message);
    }

    #[test]
    fn cd_must_be_absolute() {
        let err = parse("cd tmp\ncargo test\n").expect_err("should fail");
        assert!(err.message.contains("absolute"), "got: {}", err.message);
    }

    #[test]
    fn cd_with_and_operators_is_refused() {
        let err = parse("cd /tmp && cargo test\n").expect_err("should fail");
        assert!(err.message.contains("&&"), "got: {}", err.message);
        assert_eq!(err.line, 1);
    }

    #[test]
    fn cd_with_semicolon_is_refused() {
        let err = parse("cd /tmp; cargo test\n").expect_err("should fail");
        assert!(err.message.contains(";"), "got: {}", err.message);
        assert_eq!(err.line, 1);
    }

    #[test]
    fn cd_with_pipe_is_refused() {
        let err = parse("cd /tmp | cargo test\n").expect_err("should fail");
        assert!(err.message.contains('|'), "got: {}", err.message);
        assert_eq!(err.line, 1);
    }

    #[test]
    fn hidden_rejects_create_replace_overwrite_append() {
        let cases = [
            "create /srv/secret/a.rs\n@new\nx\n@end\n",
            "replace /srv/secret/a.rs\n@old\nx\n@new\ny\n@end\n",
            "overwrite /srv/secret/a.rs\n@new\nx\n@end\n",
            "append /srv/secret/a.rs\n@new\nx\n@end\n",
        ];
        for c in cases {
            let err = parse_hidden(c, &["/srv/secret"]).expect_err("should fail");
            assert!(
                err.message.starts_with("deliberately hidden by the user: "),
                "got: {}",
                err.message
            );
            assert_eq!(err.line, 1);
        }
    }

    #[test]
    fn hidden_rejects_delete_zapmv_mkdir_rmdir_chmod() {
        let cases = [
            "delete /srv/secret/a.rs\n",
            "zapmv /srv/secret/a.rs /tmp/b.rs\n",
            "zapmv /tmp/a.rs /srv/secret/b.rs\n",
            "mkdir /srv/secret/sub\n",
            "rmdir /srv/secret\n",
            "chmod +x /srv/secret/a.sh\n",
        ];
        for c in cases {
            let err = parse_hidden(c, &["/srv/secret"]).expect_err("should fail");
            assert!(
                err.message.starts_with("deliberately hidden by the user: "),
                "got: {} for {}",
                err.message,
                c
            );
        }
    }

    #[test]
    fn hidden_rejects_delete_with_hidden_path_second() {
        let err = parse_hidden("delete /tmp/a.rs /srv/secret/b.rs\n", &["/srv/secret"])
            .expect_err("should fail");
        assert!(
            err.message.starts_with("deliberately hidden by the user: "),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn hidden_rejects_cd_into_hidden_folder() {
        let err = parse_hidden("cd /srv/secret\ncargo test\n", &["/srv/secret"])
            .expect_err("should fail");
        assert!(
            err.message.starts_with("deliberately hidden by the user: "),
            "got: {}",
            err.message
        );
        assert_eq!(err.line, 1);
    }

    #[test]
    fn hidden_rejects_run_on_hidden_file() {
        let err = parse_hidden("cd /tmp\nrun /srv/secret/x.sh\n", &["/srv/secret"])
            .expect_err("should fail");
        assert!(
            err.message.starts_with("deliberately hidden by the user: "),
            "got: {}",
            err.message
        );
        assert_eq!(err.line, 2);
    }

    #[test]
    fn empty_config_keeps_every_form_parsing() {
        let cases = [
            "create /srv/secret/a.rs\n@new\nx\n@end\n",
            "delete /srv/secret/a.rs\n",
            "mkdir /srv/secret/sub\n",
            "cd /srv/secret\ncargo test\n",
        ];
        for c in cases {
            parse_with(c, &ParseConfig::empty()).expect("should parse");
        }
    }

    #[test]
    fn allowlist_passes_allowed_extensions_in_chain() {
        let config = allowlist_config("rs, md");
        for c in [
            "create /tmp/a.rs\n@new\nx\n@end\n",
            "create ~/a.MD\n@new\nx\n@end\n",
        ] {
            parse_with(c, &config).expect("should parse");
        }
    }

    #[test]
    fn allowlist_refuses_write_verbs_on_other_extensions() {
        let config = allowlist_config("rs, md");
        for verb in ["create", "replace", "overwrite", "append"] {
            let body = if verb == "replace" {
                format!("{verb} /tmp/a.txt\n@old\nx\n@new\ny\n@end\n")
            } else {
                format!("{verb} /tmp/a.txt\n@new\nx\n@end\n")
            };
            let err = parse_cfg(&body, &config).expect_err("should refuse");
            assert!(
                err.message.contains(".txt"),
                "verb {verb}, got: {}",
                err.message
            );
            assert_eq!(err.line, 1);
        }
        for path in ["/tmp/Makefile", "/tmp/.gitignore", "/tmp/a."] {
            let body = format!("create {path}\n@new\nx\n@end\n");
            let err = parse_cfg(&body, &config).expect_err("should refuse");
            assert!(
                err.message.contains("no extension"),
                "{path}: got {}",
                err.message
            );
        }
    }

    #[test]
    fn allowlist_does_not_affect_non_writing_verbs() {
        let config = allowlist_config("rs");
        for c in [
            "delete /tmp/Makefile\n",
            "zapmv /tmp/Makefile /tmp/b\n",
            "mkdir /tmp/Makefile\n",
            "rmdir /tmp/Makefile\n",
            "chmod +x /tmp/Makefile\n",
            "cd /tmp\nrun /tmp/x.sh\n",
            "cd /tmp\ncargo check\n",
        ] {
            parse_with(c, &config).expect("should parse");
        }
    }

    #[test]
    fn broken_allowlist_refuses_and_names_the_file() {
        let config = allowlist_config(".rs;.md");
        let err = parse_cfg("create /tmp/a.rs\n@new\nx\n@end\n", &config)
            .expect_err("should refuse");
        assert!(err.message.contains("fix the file"), "got: {}", err.message);
    }

    // ---- collect-all parse errors (step 11) ----

    #[test]
    fn error_clause_ends_with_its_line() {
        let f = parse_all("rm /tmp/a.rs\n").expect_err("should fail");
        assert_eq!(f.to_string(), "use delete at line 1");
    }

    #[test]
    fn three_independent_mistakes_make_one_sentence() {
        let input = "rm /tmp/a.rs\ncreate src/a.rs\n@new\nx\n@end\ncd tmp\ncargo test\n";
        let f = parse_all(input).expect_err("should fail");
        let s = f.to_string();
        assert_eq!(
            s,
            "use delete at line 1; create: path must be absolute or start with ~/ (got src/a.rs) at line 2; cd: folder must be absolute or start with ~/ (got tmp) at line 6"
        );
        assert!(!s.contains('\n'));
        assert!(!s.contains("line 1:"));
    }

    #[test]
    fn missing_end_then_good_block_gives_one_error() {
        let input = "create /tmp/a.rs\n@new\nfoo\ncreate /tmp/b.rs\n@new\nbar\n@end\n";
        let f = parse_all(input).expect_err("should fail");
        assert_eq!(f.errors().len(), 1, "got: {f}");
        assert_eq!(f.first().line, 5);
        assert!(!f.to_string().contains("unknown command"));
    }

    #[test]
    fn misspelled_verb_with_body_gives_one_error() {
        let f = parse_all("craete /tmp/a.rs\n@new\nx\n@end\n").expect_err("should fail");
        assert_eq!(f.errors().len(), 1, "got: {f}");
        assert!(f.first().message.contains("unknown command"));
    }

    #[test]
    fn forgotten_body_skips_code_lines() {
        let input = "create /tmp/a.rs\nfn main() {}\nlet x = 1;\ndelete src/b.rs\n";
        let f = parse_all(input).expect_err("should fail");
        assert_eq!(f.errors().len(), 2, "got: {f}");
        assert_eq!(f.errors()[0].line, 2);
        assert!(f.errors()[0].message.contains("expected @new"));
        assert_eq!(f.errors()[1].line, 4);
        assert!(f.errors()[1].message.contains("absolute"));
    }

    #[test]
    fn bad_cd_with_good_cargo_gives_one_error() {
        let f = parse_all("cd tmp\ncargo test\n").expect_err("should fail");
        assert_eq!(f.errors().len(), 1, "got: {f}");
        assert!(f.first().message.contains("absolute"));
    }

    #[test]
    fn stray_cd_before_create_is_reported_once() {
        let good = "cd /tmp\ncreate /tmp/a.rs\n@new\nx\n@end\n";
        let f = parse_all(good).expect_err("should fail");
        assert_eq!(f.errors().len(), 1, "got: {f}");
        assert_eq!(f.first().line, 1);
        assert!(f.first().message.contains("cd must be followed"));

        let bad = "cd /tmp\ncreate src/a.rs\n@new\nx\n@end\n";
        let f = parse_all(bad).expect_err("should fail");
        assert_eq!(f.errors().len(), 2, "got: {f}");
        assert_eq!(f.errors()[0].line, 1);
        assert_eq!(f.errors()[1].line, 2);
        assert!(f.errors()[1].message.contains("absolute"));
    }

    #[test]
    fn steps_after_run_give_one_complaint() {
        let input = "cd /tmp\ncargo test\ndelete /tmp/a\ndelete /tmp/b\nmkdir /tmp/c\nrmdir /tmp/d\n";
        let f = parse_all(input).expect_err("should fail");
        assert_eq!(f.errors().len(), 1, "got: {f}");
        assert_eq!(f.first().line, 3);
        assert!(f.first().message.contains("last step"));
    }

    #[test]
    fn errors_after_run_still_show() {
        let input = "cd /tmp\ncargo test\ndelete /tmp/a\ndelete rel\n";
        let f = parse_all(input).expect_err("should fail");
        assert_eq!(f.errors().len(), 2, "got: {f}");
        assert!(f.errors()[0].message.contains("last step"));
        assert!(f.errors()[1].message.contains("absolute"));
    }

    #[test]
    fn twenty_five_bad_lines_are_capped_at_twenty() {
        let input: String = (1..=25).map(|n| format!("rm /tmp/a{n}\n")).collect();
        let f = parse_all(&input).expect_err("should fail");
        assert_eq!(f.errors().len(), 25);
        let s = f.to_string();
        assert_eq!(s.matches(" at line ").count(), 20, "got: {s}");
        assert!(s.ends_with(" (+ 5 more)"), "got: {s}");
    }

    #[test]
    fn identical_errors_are_deduplicated() {
        let one = ParseError { line: 3, message: "x".to_string() };
        let f = ParseFailure::from_errors(vec![one.clone(), one]).expect("non-empty");
        assert_eq!(f.errors().len(), 1);
        assert!(ParseFailure::from_errors(Vec::new()).is_none());
    }

    #[test]
    fn from_word_knows_every_verb_and_not_rm_mv() {
        for w in [
            "create", "replace", "overwrite", "append", "delete", "zapmv", "mkdir",
            "rmdir", "chmod", "cargo", "run", "rename", "astgrep",
        ] {
            let v = Verb::from_word(w).expect(w);
            assert_eq!(v.name(), w);
        }
        assert!(Verb::from_word("rm").is_none());
        assert!(Verb::from_word("mv").is_none());
        assert!(Verb::from_word("cd").is_none());
    }

    #[test]
    fn parses_a_rename() {
        let steps = parse("rename /tmp/a.rs b.rs\n").expect("parse");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].verb, Verb::Rename);
        assert_eq!(steps[0].path, "/tmp/a.rs");
        assert_eq!(steps[0].dest.as_deref(), Some("/tmp/b.rs"));
    }

    #[test]
    fn parses_rename_at_root() {
        let steps = parse("rename /a.rs b.rs\n").expect("parse");
        assert_eq!(steps[0].dest.as_deref(), Some("/b.rs"));
    }

    #[test]
    fn rename_refuses_a_slash_in_new_name() {
        let err = parse("rename /tmp/a.rs sub/b.rs\n").expect_err("should fail");
        assert!(err.message.contains("bare file name"), "got: {}", err.message);
    }

    #[test]
    fn rename_refuses_dotdot_as_new_name() {
        let err = parse("rename /tmp/a.rs ..\n").expect_err("should fail");
        assert!(err.message.contains("bare file name"), "got: {}", err.message);
    }

    #[test]
    fn rename_refuses_same_name() {
        let err = parse("rename /tmp/a.rs a.rs\n").expect_err("should fail");
        assert!(err.message.contains("same as the old one"), "got: {}", err.message);
    }

    #[test]
    fn rename_refuses_one_or_three_args() {
        let err = parse("rename /tmp/a.rs\n").expect_err("should fail");
        assert!(err.message.contains("needs a path and a new name"), "got: {}", err.message);
        let err = parse("rename /tmp/a.rs b.rs c.rs\n").expect_err("should fail");
        assert!(err.message.contains("needs a path and a new name"), "got: {}", err.message);
    }

    #[test]
    fn rename_refuses_flags() {
        let err = parse("rename --force /tmp/a.rs b.rs\n").expect_err("should fail");
        assert!(err.message.contains("flags are not supported"), "got: {}", err.message);
    }

    #[test]
    fn rename_refuses_a_relative_path() {
        let err = parse("rename a.rs b.rs\n").expect_err("should fail");
        assert!(err.message.contains("absolute"), "got: {}", err.message);
    }

    #[test]
    fn rename_refuses_trailing_slash() {
        let err = parse("rename /tmp/a/ b.rs\n").expect_err("should fail");
        assert!(err.message.contains("path must name a file"), "got: {}", err.message);
    }

    #[test]
    fn rename_accepts_an_extensionless_name_when_allowlist_is_set() {
        let config = allowlist_config("rs");
        let steps = parse_with("rename /tmp/Makefile Makefile2\n", &config).expect("parse");
        assert_eq!(steps[0].verb, Verb::Rename);
    }

    #[test]
    fn parses_an_astgrep_rename() {
        let steps = parse("astgrep rename /tmp/proj OldName NewName\n").expect("parse");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].verb, Verb::AstGrep);
        assert_eq!(steps[0].path, "/tmp/proj");
        assert_eq!(steps[0].old_text, "OldName");
        assert_eq!(steps[0].new_text, "NewName");
        assert!(steps[0].dest.is_none());
    }

    #[test]
    fn astgrep_takes_a_home_folder() {
        let steps = parse("astgrep rename ~/proj a b\n").expect("parse");
        assert_eq!(steps[0].path, "~/proj");
    }

    #[test]
    fn astgrep_only_knows_rename() {
        let err = parse("astgrep scan /tmp/p a b\n").expect_err("should fail");
        assert!(err.message.contains("only rename"), "got: {}", err.message);
    }

    #[test]
    fn astgrep_needs_a_folder_and_two_names() {
        let err = parse("astgrep rename /tmp/p a\n").expect_err("should fail");
        assert!(err.message.contains("needs rename"), "got: {}", err.message);
    }

    #[test]
    fn astgrep_refuses_bad_names_on_either_side() {
        for bad in ["1abc", "a-b", "_", "a.b", "match", "self", "Self", "fn", "\u{e9}t\u{e9}"] {
            let old_bad = format!("astgrep rename /tmp/p {bad} fine_name\n");
            let err = parse(&old_bad).expect_err("should fail");
            assert!(err.message.contains("the old name"), "{bad}: got {}", err.message);
            let new_bad = format!("astgrep rename /tmp/p fine_name {bad}\n");
            let err = parse(&new_bad).expect_err("should fail");
            assert!(err.message.contains("the new name"), "{bad}: got {}", err.message);
        }
    }

    #[test]
    fn astgrep_accepts_underscore_names() {
        parse("astgrep rename /tmp/p _a b_ \n").expect("parse");
        parse("astgrep rename /tmp/p __x Y9\n").expect("parse");
    }

    #[test]
    fn astgrep_refuses_the_same_name() {
        let err = parse("astgrep rename /tmp/p foo foo\n").expect_err("should fail");
        assert!(err.message.contains("same as the old one"), "got: {}", err.message);
    }

    #[test]
    fn astgrep_refuses_a_relative_folder() {
        let err = parse("astgrep rename proj a b\n").expect_err("should fail");
        assert!(err.message.contains("absolute"), "got: {}", err.message);
    }

    #[test]
    fn astgrep_refuses_a_hidden_folder() {
        let err = parse_hidden("astgrep rename /srv/secret/p a b\n", &["/srv/secret"])
            .expect_err("should fail");
        assert!(
            err.message.starts_with("deliberately hidden by the user: "),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn two_astgrep_steps_in_one_chain_are_refused() {
        let f = parse_all("astgrep rename /tmp/p a b\nastgrep rename /tmp/q c d\n")
            .expect_err("should fail");
        assert_eq!(f.errors().len(), 1, "got: {f}");
        assert_eq!(f.first().line, 2);
        assert!(f.first().message.contains("only one astgrep"));
    }

    #[test]
    fn astgrep_may_be_followed_by_cd_and_cargo() {
        let steps =
            parse("astgrep rename /tmp/p a b\ncd /tmp/p\ncargo check\n").expect("parse");
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].verb, Verb::AstGrep);
        assert_eq!(steps[1].verb, Verb::Cargo);
    }

    #[test]
    fn astgrep_ignores_the_extension_allowlist() {
        let config = allowlist_config("md");
        parse_with("astgrep rename /tmp/p a b\n", &config).expect("parse");
    }

    #[test]
    fn keywords_are_all_refused_as_identifiers() {
        for k in RUST_KEYWORDS {
            assert!(identifier_problem(k).is_some(), "{k}");
        }
        assert!(identifier_problem("foo").is_none());
        assert!(identifier_problem("r#foo").is_some());
    }

    #[test]
    fn every_verb_name_appears_in_the_cheat_sheet() {
        // Drift guard (step 17): if a verb is added but the cheat sheet
        // (which is what the AI is told to read) is not updated, this
        // fails.
        let sheet = include_str!("../../CHAIN_CHEATSHEET.md");
        for v in Verb::ALL {
            assert!(
                sheet.contains(v.name()),
                "verb {:?} ({}) not named in CHAIN_CHEATSHEET.md",
                v,
                v.name()
            );
        }
    }

    #[test]
    fn verb_all_round_trips_name_and_flags() {
        for v in Verb::ALL {
            assert_eq!(Verb::from_word(v.name()), Some(v), "name round-trip {:?}", v);
        }
        let expect_special = |v: Verb| {
            matches!(
                v,
                Verb::Cargo | Verb::Run | Verb::Chmod | Verb::Rename | Verb::AstGrep
            )
        };
        let expect_always = |v: Verb| {
            matches!(
                v,
                Verb::Remove | Verb::Move | Verb::RemoveDir | Verb::Chmod | Verb::Cargo | Verb::Run | Verb::Rename | Verb::AstGrep
            )
        };
        let expect_editable = |v: Verb| {
            matches!(v, Verb::Create | Verb::Mkdir | Verb::Move | Verb::Rename)
        };
        for v in Verb::ALL {
            assert_eq!(v.is_special(), expect_special(v), "is_special {:?}", v);
            assert_eq!(v.always_reviewed(), expect_always(v), "always_reviewed {:?}", v);
            assert_eq!(v.has_editable_target(), expect_editable(v), "has_editable_target {:?}", v);
        }
    }
}

#[cfg(test)]
mod quote_tests {
    use super::*;

    fn parse_q(input: &str) -> Result<Vec<Step>, ParseError> {
        parse_with(input, &ParseConfig::empty()).map_err(|f| f.first().clone())
    }

    #[test]
    fn quoted_create_path_is_stripped() {
        let steps = parse_q("create \"/tmp/a b.md\"\n@new\nx\n@end\n").expect("parse");
        assert_eq!(steps[0].path, "/tmp/a b.md");
    }

    #[test]
    fn single_quoted_create_path_is_stripped() {
        let steps = parse_q("create '/tmp/a b.md'\n@new\nx\n@end\n").expect("parse");
        assert_eq!(steps[0].path, "/tmp/a b.md");
    }

    #[test]
    fn quoted_cd_folder_is_stripped() {
        let steps =
            parse_q("cd \"/tmp/a b\"\ncargo check\n").expect("parse");
        assert_eq!(steps[0].cwd.as_deref(), Some("/tmp/a b"));
    }

    #[test]
    fn quoted_append_path_is_stripped() {
        let steps = parse_q("append \"/tmp/a b.md\"\n@new\nmore\n@end\n").expect("parse");
        assert_eq!(steps[0].path, "/tmp/a b.md");
    }

    #[test]
    fn quoted_overwrite_path_is_stripped() {
        let steps =
            parse_q("overwrite \"/tmp/a b.md\"\n@new\nwhole\n@end\n").expect("parse");
        assert_eq!(steps[0].path, "/tmp/a b.md");
    }

    #[test]
    fn quoted_replace_path_is_stripped() {
        let steps = parse_q(
            "replace \"/tmp/a b.md\"\n@old\nx\n@new\ny\n@end\n",
        )
        .expect("parse");
        assert_eq!(steps[0].path, "/tmp/a b.md");
    }

    #[test]
    fn quote_with_count_flag_still_works() {
        let steps = parse_q(
            "replace \"/tmp/a b.md\" x3\n@old\nx\n@new\ny\n@end\n",
        )
        .expect("parse");
        assert_eq!(steps[0].path, "/tmp/a b.md");
        assert_eq!(steps[0].count, Some(3));
    }

    #[test]
    fn quote_with_no_newline_flag_still_works() {
        let steps =
            parse_q("create \"/tmp/a b.md\" --no-newline\n@new\nx\n@end\n")
                .expect("parse");
        assert_eq!(steps[0].path, "/tmp/a b.md");
        assert!(steps[0].no_newline);
    }

    #[test]
    fn mismatched_quotes_are_still_refused() {
        // `"/tmp/a` never closes the quote; the leading `"` makes the
        // path look relative, so the absolute check fires.
        let err = parse_q("create \"/tmp/a b.md\n@new\nx\n@end\n")
            .expect_err("should fail");
        assert!(err.message.contains("absolute"), "got: {}", err.message);
    }

    #[test]
    fn empty_quoted_path_is_missing() {
        let err = parse_q("create \"\"\n@new\nx\n@end\n").expect_err("should fail");
        assert!(err.message.contains("missing path"), "got: {}", err.message);
    }

    #[test]
    fn unquoted_path_with_space_is_still_accepted() {
        // The old behaviour: everything after the verb is the path.
        let steps = parse_q("create /tmp/a b.md\n@new\nx\n@end\n").expect("parse");
        assert_eq!(steps[0].path, "/tmp/a b.md");
    }

    #[test]
    fn strip_one_quote_pair_works() {
        assert_eq!(strip_one_quote_pair("\"/a b\""), "/a b");
        assert_eq!(strip_one_quote_pair("'/a b'"), "/a b");
        assert_eq!(strip_one_quote_pair("/a b"), "/a b");
        assert_eq!(strip_one_quote_pair("\"/a"), "\"/a");
        assert_eq!(strip_one_quote_pair("a\""), "a\"");
        assert_eq!(strip_one_quote_pair("\"a'"), "\"a'");
        assert_eq!(strip_one_quote_pair(""), "");
        assert_eq!(strip_one_quote_pair("\""), "\"");
    }
}
