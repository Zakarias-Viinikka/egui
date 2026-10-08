use crate::text_diff::DiffRowProcessedForUi;

pub struct PendingOverwrite {
    pub path: String,
    pub old_content: String,
    pub new_content: String,
    pub rows: Vec<DiffRowProcessedForUi>,
}

impl PendingOverwrite {
    /// Write new_content to path, but only if the file on disk still matches
    /// what the diff was computed against. Returns Err with the reason.
    pub fn confirm(&self) -> Result<(), String> {
        let current = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("couldn't read the file: {e}"))?;
        if current != self.old_content {
            return Err("file changed since the diff was made".to_string());
        }
        std::fs::write(&self.path, &self.new_content)
            .map_err(|e| format!("write failed: {e}"))?;
        Ok(())
    }

    /// Copy the denial sentence to the clipboard. Write nothing to disk.
    pub fn deny(&self) -> Result<(), String> {
        let msg = format!("denied request to edit file {}", self.path);
        crate::clipboard::copy_to_clipboard(&msg)
    }
}
