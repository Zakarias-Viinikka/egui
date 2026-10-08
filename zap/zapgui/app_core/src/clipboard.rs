use std::io::Write;
use std::process::{Command, Stdio};

/// Copy text to the system clipboard using xclip. Returns Err with a
/// human-readable reason if xclip can't start or exits non-zero, so a failed
/// copy can't look like success.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("xclip could not start: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("xclip write failed: {e}"))?;
    }

    let status = child.wait().map_err(|e| format!("xclip wait failed: {e}"))?;
    if !status.success() {
        return Err(format!("xclip exited with {status}"));
    }
    Ok(())
}
