use std::fmt;

#[derive(Debug, Clone)]
pub struct AppError {
    pub err_msg: String,
    pub method: String,
    pub file: String,
}

impl AppError {
    pub fn new(err_msg: impl Into<String>, method: impl Into<String>, file: impl Into<String>) -> Self {
        Self { err_msg: err_msg.into(), method: method.into(), file: file.into() }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}: {}", self.file, self.method, self.err_msg)
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, AppError>;
