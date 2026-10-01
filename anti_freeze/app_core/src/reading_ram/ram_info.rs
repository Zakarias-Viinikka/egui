#[derive(Clone)]
pub struct RamInfo {
    pub name: String,
    pub pid: u32,
    pub memory_bytes: u64,
    pub id: u64,
}

impl RamInfo {
    pub fn human_memory(&self) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        let b = self.memory_bytes as f64;
        if b >= GB {
            format!("{:.2} GB", b / GB)
        } else if b >= MB {
            format!("{:.1} MB", b / MB)
        } else if b >= KB {
            format!("{:.0} KB", b / KB)
        } else {
            format!("{} B", self.memory_bytes)
        }
    }
}
