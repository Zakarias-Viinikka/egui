//! Chain grammar: parse a chain block into a list of steps.
//! Shared by zapcli (which parses before sending, so bad paths and
//! disallowed cargo args never reach the socket) and zapgui (which parses
//! again and enforces the same rules).
//!
//! A chain may end with one run step: a `cd <folder>` line, then a
//! `cargo <args>` or `run <file> [args]` line. The cd is a directive, not
//! a step: it sets the working folder for the run step after it.

use crate::argv;
use crate::cargo_gate;

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
}

impl Verb {
    pub fn name(self) -> &'static str {
        match self {
            Verb::Create => "create",
            Verb::Replace => "replace",
            Verb::Overwrite => "overwrite",
            Verb::Append => "append",
            Verb::Remove => "zaprm",
            Verb::Move => "zapmv",
            Verb::Mkdir => "mkdir",
            Verb::RemoveDir => "rmdir",
            Verb::Cargo => "cargo",
            Verb::Run => "run",
        }
    }

    pub fn is_run(self) -> bool {
        matches!(self, Verb::Cargo | Verb::Run)
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

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<Vec<Step>, ParseError> {
    let lines: Vec<&str> = input.lines().collect();
    let mut steps: Vec<Step> = Vec::new();
    let mut i = 0;
    let mut pending_cwd: Option<(String, usize)> = None;
    let mut saw_run = false;

    while i < lines.len() {
        if lines[i].trim_end().is_empty() {
            i += 1;
            continue;
        }
        let header_no = i + 1;
        let line = lines[i].trim_end();

        // cd is a directive for a following run step, not a step itself.
        if let Some(folder) = try_cd(line) {
            if saw_run {
                return Err(ParseError {
                    line: header_no,
                    message: "a run step must be the last step".to_string(),
                });
            }
            if pending_cwd.is_some() {
                return Err(ParseError {
                    line: header_no,
                    message: "cd must be followed by cargo or run".to_string(),
                });
            }
            if !is_absolute_or_home(folder) {
                return Err(ParseError {
                    line: header_no,
                    message: format!(
                        "cd: folder must be absolute or start with ~/ (got {folder})"
                    ),
                });
            }
            pending_cwd = Some((folder.to_string(), header_no));
            i += 1;
            continue;
        }

        let (verb, rest) = parse_header(line, header_no)?;
        i += 1;

        if saw_run {
            return Err(ParseError {
                line: header_no,
                message: "a run step must be the last step".to_string(),
            });
        }

        match verb {
            Verb::Remove | Verb::Move | Verb::Mkdir | Verb::RemoveDir => {
                if let Some((_, cd_line)) = &pending_cwd {
                    return Err(ParseError {
                        line: *cd_line,
                        message: "cd must be followed by cargo or run".to_string(),
                    });
                }
                let new_steps = parse_single_line(verb, rest, header_no)?;
                steps.extend(new_steps);
            }
            Verb::Create | Verb::Replace | Verb::Overwrite | Verb::Append => {
                if let Some((_, cd_line)) = &pending_cwd {
                    return Err(ParseError {
                        line: *cd_line,
                        message: "cd must be followed by cargo or run".to_string(),
                    });
                }
                if i >= lines.len() {
                    return Err(ParseError {
                        line: header_no,
                        message: format!("{}: expected @new", verb.name()),
                    });
                }
                if !is_marker_line(lines[i]) {
                    return Err(ParseError {
                        line: i + 1,
                        message: format!("{}: expected @new", verb.name()),
                    });
                }
                let first_name = marker_name(lines[i]).to_string();
                if verb == Verb::Replace {
                    if first_name != "old" {
                        return Err(ParseError {
                            line: i + 1,
                            message: format!("replace: expected @old, got @{}", first_name),
                        });
                    }
                } else if first_name != "new" {
                    return Err(ParseError {
                        line: i + 1,
                        message: format!(
                            "{}: expected @new, got @{}",
                            verb.name(),
                            first_name
                        ),
                    });
                }

                let (old_text, new_text, next_i) = if verb == Verb::Replace {
                    let (old_text, after_old) = read_body(&lines, i + 1, "new")?;
                    let (new_text, after_new) = read_body(&lines, after_old, "end")?;
                    (old_text, new_text, after_new)
                } else {
                    let (new_text, after_new) = read_body(&lines, i + 1, "end")?;
                    (String::new(), new_text, after_new)
                };

                let (path, count, no_newline) = parse_path_and_flags(rest, header_no, verb)?;

                let new_text = if verb == Verb::Create && !no_newline {
                    let mut t = new_text;
                    if !t.ends_with('\n') {
                        t.push('\n');
                    }
                    t
                } else {
                    new_text
                };

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
                i = next_i;
            }
            Verb::Cargo | Verb::Run => {
                let (folder, _cd_line) = pending_cwd.take().ok_or_else(|| ParseError {
                    line: header_no,
                    message: format!(
                        "{}: needs a preceding 'cd <folder>' line",
                        verb.name()
                    ),
                })?;
                let args = argv::split(rest.trim()).map_err(|e| ParseError {
                    line: header_no,
                    message: format!("{}: {}", verb.name(), e),
                })?;
                if args.is_empty() {
                    return Err(ParseError {
                        line: header_no,
                        message: format!("{}: missing arguments", verb.name()),
                    });
                }
                if verb == Verb::Cargo {
                    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    if let Err(e) = cargo_gate::check(&refs) {
                        return Err(ParseError { line: header_no, message: e });
                    }
                } else {
                    if !is_absolute_or_home(&args[0]) {
                        return Err(ParseError {
                            line: header_no,
                            message: format!(
                                "run: file must be absolute or start with ~/ (got {})",
                                args[0]
                            ),
                        });
                    }
                }
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
                saw_run = true;
            }
        }
    }

    if let Some((_, cd_line)) = pending_cwd {
        return Err(ParseError {
            line: cd_line,
            message: "cd must be followed by cargo or run".to_string(),
        });
    }

    Ok(steps)
}

fn try_cd(line: &str) -> Option<&str> {
    let (word, rest) = match line.find(char::is_whitespace) {
        Some(idx) => (&line[..idx], line[idx + 1..].trim()),
        None => (line, ""),
    };
    if word == "cd" && !rest.is_empty() {
        Some(rest)
    } else {
        None
    }
}

fn parse_header(line: &str, line_no: usize) -> Result<(Verb, &str), ParseError> {
    let (word, rest) = match line.find(char::is_whitespace) {
        Some(idx) => (&line[..idx], &line[idx + 1..]),
        None => (line, ""),
    };
    let verb = match word {
        "create" => Verb::Create,
        "replace" => Verb::Replace,
        "overwrite" => Verb::Overwrite,
        "append" => Verb::Append,
        "zaprm" => Verb::Remove,
        "zapmv" => Verb::Move,
        "rm" => {
            return Err(ParseError {
                line: line_no,
                message: "use zaprm".to_string(),
            })
        }
        "mv" => {
            return Err(ParseError {
                line: line_no,
                message: "use zapmv".to_string(),
            })
        }
        "mkdir" => Verb::Mkdir,
        "rmdir" => Verb::RemoveDir,
        "cargo" => Verb::Cargo,
        "run" => Verb::Run,
        _ => {
            return Err(ParseError {
                line: line_no,
                message: format!("unknown command: {:?}", word),
            })
        }
    };
    if rest.trim().is_empty() {
        return Err(ParseError {
            line: line_no,
            message: format!("{}: missing path", verb.name()),
        });
    }
    Ok((verb, rest))
}

fn parse_single_line(verb: Verb, rest: &str, line: usize) -> Result<Vec<Step>, ParseError> {
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
                        message: format!("rm: unsupported option {a}"),
                    });
                }
                require_absolute(a, line, verb)?;
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
            require_absolute(&args[0], line, verb)?;
            require_absolute(&args[1], line, verb)?;
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
                require_absolute(a, line, verb)?;
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
                require_absolute(a, line, verb)?;
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
        _ => unreachable!(),
    }
}

fn require_absolute(path: &str, line: usize, verb: Verb) -> Result<(), ParseError> {
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
    Ok(())
}

fn is_absolute_or_home(p: &str) -> bool {
    p == "~" || p.starts_with("~/") || p.starts_with('/')
}

fn parse_path_and_flags(
    rest: &str,
    line: usize,
    verb: Verb,
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
        let steps = parse("zaprm /tmp/a.rs\n").expect("parse");
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
        assert!(err.message.contains("zaprm"), "got: {}", err.message);
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
}
