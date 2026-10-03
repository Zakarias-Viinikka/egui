use super::super::{Outcome, reply_outcome};
use crate::general_util::socket;

pub fn run(rest: &str) -> Outcome {
    reply_outcome(socket::send(rest))
}
