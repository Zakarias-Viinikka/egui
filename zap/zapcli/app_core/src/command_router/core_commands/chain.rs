use super::super::{Outcome, Session, reply_outcome};

/// Forward a chain block to zapgui. `input` is what the user typed,
/// verbatim, including any newlines from a paste.
pub fn run(input: &str, session: &Session) -> Outcome {
    let cwd = session.cwd.display().to_string();
    let payload = format!("chain {}\n{}", cwd, input);
    reply_outcome(crate::general_util::socket::send(&payload))
}
