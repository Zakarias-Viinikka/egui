use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static SESSION_ID: OnceLock<String> = OnceLock::new();

pub fn get() -> &'static str {
    SESSION_ID.get_or_init(generate)
}

fn generate() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("session_{}", millis)
}
