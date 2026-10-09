use super::super::{Outcome, reply_outcome};

pub fn run() -> Outcome {
    reply_outcome(crate::general_util::socket::send("menu"))
}
