use super::super::{Line, Outcome, Session, reply_outcome};

/// Forward a chain block to zapgui. Parse first, so a bad path or an
/// unknown verb never leaves the socket. `input` is what the user typed,
/// verbatim, including any newlines from a paste.
pub fn run(input: &str, session: &Session) -> Outcome {
    let steps = match zap_wire::chain::parse(input) {
        Ok(s) => s,
        Err(e) => {
            return Outcome::Print(vec![Line::error(format!(
                "zap: nothing was sent. {e}."
            ))])
        }
    };
    let cwd = session.cwd.display().to_string();
    let payload = format!("chain {}\n{}", cwd, input);
    if steps.iter().any(|s| s.verb.is_run()) {
        let run_payload = format!("{}\n{}", cwd, input);
        return match crate::general_util::socket::open_chain_run(&run_payload) {
            Ok(sess) => Outcome::Streaming(sess),
            Err(e) => Outcome::Print(vec![Line::error(e)]),
        };
    }
    reply_outcome(crate::general_util::socket::send(&payload))
}