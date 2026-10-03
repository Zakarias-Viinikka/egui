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
}

pub struct Line {
    pub text: String,
    pub style: LineStyle,
}

impl Line {
    pub fn normal(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Normal } }
    pub fn error(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Error } }
    pub fn folder(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Folder } }
    pub fn prompt(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Prompt } }
    pub fn zap(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::Zap } }
    pub fn cli(text: impl Into<String>) -> Self { Self { text: text.into(), style: LineStyle::CliMessage } }
}

pub enum Outcome {
    Print(Vec<Line>),
    Silent,
    Unknown,
    Exit,
    Clear,
    CopyAll,
    PrintAndCopy { lines: Vec<Line>, text: String },
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
    if let Some(rest) = trimmed.strip_prefix("overwrite ") {
        return core_commands::overwrite::run(rest, session);
    }

    let mut parts = trimmed.split_whitespace();
    let Some(verb) = parts.next() else {
        return Outcome::Unknown;
    };
    let args: Vec<&str> = parts.collect();

    match verb {
        "cd" => core_commands::cd::run(&args, session),
        "ls" => core_commands::ls::run(&args, session),
        "cat" => core_commands::cat::run(&args, session),
        "s" | "settings" => core_commands::settings::run(),
        "exit" | "quit" => Outcome::Exit,
        "cls" | "clear" => Outcome::Clear,
        "copy" => Outcome::CopyAll,
        _ => Outcome::Unknown,
    }
}

/// Format a reply from zapgui as history lines. zap replies render in a
/// distinct colour so the user can tell them apart from cli messages.
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
