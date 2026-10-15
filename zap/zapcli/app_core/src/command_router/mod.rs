pub mod core_commands;

use crate::general_util::path_parsing::home_dir;
use std::path::PathBuf;

pub struct Session {
    pub cwd: PathBuf,
}

impl Default for Session {
    fn default() -> Self {
        Self { cwd: default_folder() }
    }
}

#[derive(Clone, Copy)]
pub enum LineStyle {
    Normal,
    Error,
    Folder,
    /// A "~/dir > command" line, so past commands stand out from their output.
    Prompt,
    /// A reply that came back from zapgui.
    Zap,
    /// An informational message from a built-in command ("copied x to
    /// clipboard"). Not an error, not data.
    CliMessage,
    /// Output of a peek command (head, tail, wc, last). Shown in a muted
    /// violet, and skipped by copy by default.
    Peek,
}

pub struct Line {
    pub text: String,
    pub style: LineStyle,
    /// When true, `copy` skips this line. `copy all` includes it. Set by
    /// peek commands for their prompt line and every output line.
    pub skip_copy: bool,
}

impl Line {
    pub fn normal(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Normal, skip_copy: false } }
    pub fn error(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Error, skip_copy: false } }
    pub fn folder(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Folder, skip_copy: false } }
    pub fn prompt(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Prompt, skip_copy: false } }
    pub fn zap(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Zap, skip_copy: false } }
    pub fn cli(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::CliMessage, skip_copy: false } }
    pub fn peek(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Peek, skip_copy: true } }
}

pub enum Outcome {
    Print(Vec<Line>),
    /// Print lines that don't get copied with the scrollback. The caller
    /// also marks the just-pushed prompt line skip_copy.
    Peek(Vec<Line>),
    Silent,
    Unknown,
    Exit,
    Clear,
    CopyAll { include_peeks: bool },
    PrintAndCopy { lines: Vec<Line>, text: String },
    /// A connection is open and will stream frames. The UI takes the
    /// Session and drains it each frame.
    Streaming(crate::general_util::socket::Session),
}

pub fn route(input: &str, session: &mut Session) -> Outcome {
    let trimmed = input.trim();

    // zap and overwrite both send their payload to zapgui. zap sends the
    // rest of the line verbatim. overwrite sends "<path>\n<content>" where
    // content can contain newlines (from a paste).
    if let Some(rest) = trimmed.strip_prefix("zap ") {
        return reply_outcome(crate::general_util::socket::send(rest));
    }
    if trimmed == "zap" {
        return reply_outcome(crate::general_util::socket::send(""));
    }
    // Chain verbs take a multi-line block (paste). Forward the raw input,
    // not the trimmed form, so newlines survive. The bare overwrite form
    // (no marker lines) is handled further down.
    if first_word_is_chain_verb(input) {
        return core_commands::chain::run(input, session);
    }

    let mut parts = trimmed.split_whitespace();
    let Some(verb) = parts.next() else {
        return Outcome::Unknown;
    };
    let args: Vec<&str> = parts.collect();

    match verb {
        "run" => {
            let line = trimmed.strip_prefix("run").unwrap_or("").trim_start();
            let args = match zap_wire::argv::split(line) {
                Ok(a) => a,
                Err(e) => return Outcome::Print(vec![Line::error(format!("run: {e}"))]),
            };
            if args.is_empty() {
                return Outcome::Print(vec![Line::error("run: missing file")]);
            }
            match crate::general_util::socket::open_run(&session.cwd, &args) {
                Ok(sess) => Outcome::Streaming(sess),
                Err(e) => Outcome::Print(vec![Line::error(e)]),
            }
        }
        "cargo" => {
            let line = trimmed.strip_prefix("cargo").unwrap_or("").trim_start();
            let args = match zap_wire::argv::split(line) {
                Ok(a) => a,
                Err(e) => return Outcome::Print(vec![Line::error(format!("cargo: {e}"))]),
            };
            let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            if let Err(e) = zap_wire::cargo_gate::check(&refs) {
                return Outcome::Print(vec![Line::error(e)]);
            }
            match crate::general_util::socket::open_cargo(&session.cwd, &args) {
                Ok(sess) => Outcome::Streaming(sess),
                Err(e) => Outcome::Print(vec![Line::error(e)]),
            }
        }
        "devstream" => match crate::general_util::socket::open("devstream") {
            Ok(sess) => Outcome::Streaming(sess),
            Err(e) => Outcome::Print(vec![Line::error(e)]),
        },
        "error" if args.is_empty() => {
            match crate::general_util::socket::send("error") {
                Ok(reply) if reply.starts_with("zap error:") => {
                    Outcome::Print(vec![Line::error(reply)])
                }
                result => reply_outcome(result),
            }
        }
        "error" => Outcome::Print(vec![Line::error("error: takes no arguments")]),
        "cd" => core_commands::cd::run(&args, session),
        "ls" => core_commands::ls::run(&args, session),
        "cat" => core_commands::cat::run(&args, session),
        "head" => core_commands::head::run(&args, session),
        "tail" => core_commands::tail::run(&args, session),
        "wc" => core_commands::wc::run(&args, session),
        "last" => core_commands::last::run(&args, session),
        "s" | "settings" => core_commands::settings::run(),
        "help" => core_commands::help::run(),
        "whitelist" => core_commands::whitelist::run(),
        "exit" | "quit" => Outcome::Exit,
        "cls" | "clear" => Outcome::Clear,
        "copy" => {
            let all = args.first().map(|a| *a == "all").unwrap_or(false);
            Outcome::CopyAll { include_peeks: all }
        }
        _ => Outcome::Unknown,
    }
}

/// Format a reply from zapgui as history lines. zap replies render in a
/// distinct colour so the user can tell them apart from cli messages.
/// Turn a zapgui reply into peek lines, so the scrollback copier skips
/// them. Used by `last`.
pub fn reply_outcome_peek(result: Result<String, String>) -> Outcome {
    match result {
        Ok(reply) => {
            let trimmed = reply.trim_end();
            if trimmed.is_empty() {
                Outcome::Silent
            } else {
                Outcome::Peek(trimmed.lines().map(Line::peek).collect())
            }
        }
        Err(e) => Outcome::Print(vec![Line::error(e)]),
    }
}

fn first_word_is_chain_verb(input: &str) -> bool {
    // A block of two or more non-blank lines whose first word is cd,
    // cargo or run is a chain. Single-line cd/cargo/run stay the human
    // verbs and are dispatched below.
    let non_blank: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
    if non_blank.len() >= 2 {
        let first = non_blank[0]
            .trim_start()
            .split_whitespace()
            .next()
            .unwrap_or("");
        if matches!(first, "cd" | "cargo" | "run") {
            return true;
        }
    }

    // Look at the first non-blank line only. A leading blank line or a
    // leading space used to make this return false, so a paste with either
    // went to the bare-overwrite path and the markers landed in the file.
    // That path is gone; the chain parser is the only route for these
    // verbs now. A marker-less overwrite fails loudly in the parser with
    // "expected a marker line".
    let first_line = input.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let trimmed = first_line.trim_start();
    let first = trimmed.split_whitespace().next().unwrap_or("");
    if !matches!(
        first,
        "create" | "replace" | "overwrite" | "append" | "rm" | "mv" | "mkdir"
    ) {
        return false;
    }
    trimmed
        .strip_prefix(first)
        .map(|rest| rest.starts_with(' ') || rest.starts_with('\t'))
        .unwrap_or(false)
}

pub fn reply_outcome(result: Result<String, String>) -> Outcome {
    match result {
        Ok(reply) => {
            let trimmed = reply.trim_end();
            if trimmed.is_empty() {
                Outcome::Silent
            } else {
                Outcome::Print(trimmed.lines().map(Line::zap).collect())
            }
        }
        Err(e) => Outcome::Print(vec![Line::error(e)]),
    }
}

pub fn settings_file() -> PathBuf {
    home_dir().join(".config/zap/settings")
}

pub fn default_folder() -> PathBuf {
    let path = settings_file();
    if let Ok(s) = std::fs::read_to_string(&path) {
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("default_folder=") {
                let p = PathBuf::from(rest.trim());
                if p.is_dir() {
                    return p;
                }
            }
        }
    }
    home_dir()
}
