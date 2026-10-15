use super::super::{Line, Outcome, Session, reply_outcome};

/// Forward a chain block to zapgui. Parse first so invalid chains are rejected
/// before the chain request is sent. Parse errors are separately forwarded for
/// clipboard copying. `input` is what the user typed, verbatim, including any
/// newlines from a paste.
pub fn run(input: &str, session: &Session) -> Outcome {
    let steps = match zap_wire::chain::parse(input) {
        Ok(s) => s,
        Err(e) => {
            let message = format!("zap: nothing was sent. {e}.");
            let copy_request = format!("copy-error\n{message}");
            match crate::general_util::socket::send(&copy_request) {
                Ok(reply) if reply.starts_with("zap error:") => {
                    return Outcome::Print(vec![
                        Line::error(message),
                        Line::error(reply),
                    ]);
                }
                Ok(_) => {}
                Err(copy_error) => {
                    return Outcome::Print(vec![
                        Line::error(message),
                        Line::error(format!(
                            "zap: could not send the error to zapgui for clipboard copying: {copy_error}"
                        )),
                    ]);
                }
            }
            return Outcome::Print(vec![Line::error(message)]);
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