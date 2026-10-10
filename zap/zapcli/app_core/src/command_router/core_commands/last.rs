use super::super::{reply_outcome_peek, Outcome, Session};
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
    reply_outcome_peek(socket::send(&payload))
}
