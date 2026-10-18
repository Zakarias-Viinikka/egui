use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Success,
    Error,
}

pub struct LogEntry {
    pub at: Instant,
    pub kind: LogKind,
    pub text: String,
}

impl LogEntry {
    pub fn new(kind: LogKind, text: impl Into<String>) -> Self {
        Self { at: Instant::now(), kind, text: text.into() }
    }
}
