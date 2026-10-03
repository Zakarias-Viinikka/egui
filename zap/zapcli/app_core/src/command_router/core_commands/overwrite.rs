use super::super::{Line, Outcome, Session, reply_outcome};
use crate::general_util::path_parsing::resolve;
use crate::general_util::socket;

/// `overwrite <path> <content>`. The path is the first whitespace-delimited
/// token. Everything after it is content, and can contain newlines (a paste
/// lands in the input line as one string with its newlines intact, before
/// Enter submits). Whitespace runs separating path from content are trimmed,
/// but newlines inside content are preserved.
pub fn run(rest: &str, session: &Session) -> Outcome {
    let trimmed = rest.trim_start();
    if trimmed.is_empty() {
        return Outcome::Print(vec![Line::error("overwrite: missing path and content")]);
    }

    let split = trimmed
        .find(|c: char| c.is_whitespace())
        .unwrap_or(trimmed.len());
    let path = &trimmed[..split];
    let content = trimmed[split..].trim_start_matches(|c| c == ' ' || c == '\t');

    if path.is_empty() {
        return Outcome::Print(vec![Line::error("overwrite: missing path")]);
    }

    let resolved = resolve(path, &session.cwd);
    let payload = format!("overwrite {}\n{}", resolved.display(), content);

    reply_outcome(socket::send(&payload))
}
