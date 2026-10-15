use super::super::{Line, Outcome, Session};
use crate::general_util::path_parsing::resolve;
use crate::general_util::socket;

pub fn run(args: &[&str], session: &Session) -> Outcome {
    let payload = match args.first() {
        None => "last".to_string(),
        Some(raw) => {
            let resolved = resolve(raw, &session.cwd);
            format!("last {}", resolved.display())
        }
    };
    match socket::send(&payload) {
        Ok(reply) => {
            let trimmed = reply.trim_end();
            if trimmed.is_empty() {
                Outcome::Silent
            } else {
                // Peek-style lines so `copy` skips them, but the whole
                // blob still goes on the clipboard now — `last` is meant
                // to be pasted back.
                let lines: Vec<Line> = trimmed.lines().map(Line::peek).collect();
                Outcome::PrintAndCopy {
                    lines,
                    text: trimmed.to_string(),
                }
            }
        }
        Err(e) => Outcome::Print(vec![Line::error(e)]),
    }
}
