pub mod ram_info;
pub mod tmp_tester;
pub use ram_info::RamInfo;

pub const TOP_N: usize = 50;

pub fn read_top_processes() -> Result<Vec<RamInfo>, String> {
    let entries = std::fs::read_dir("/proc").map_err(|e| {
        format!(
            "err_msg: read_dir /proc failed: {e} | method: read_top_processes | file: app_core/src/reading_ram/mod.rs"
        )
    })?;

    let mut out: Vec<RamInfo> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let fname = entry.file_name();
        let name_str = match fname.to_str() {
            Some(s) => s,
            None => continue,
        };
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let contents = match std::fs::read_to_string(entry.path().join("status")) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut name = String::new();
        let mut rss_kb: u64 = 0;
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("Name:") {
                name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("VmRSS:") {
                if let Some(first) = rest.split_whitespace().next() {
                    rss_kb = first.parse().unwrap_or(0);
                }
            }
        }
        if name.is_empty() || rss_kb == 0 {
            continue;
        }
        let id = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            name.hash(&mut h);
            h.finish()
        };
        out.push(RamInfo {
            name,
            pid,
            memory_bytes: rss_kb * 1024,
            id,
        });
    }
    out.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    out.truncate(TOP_N);
    Ok(out)
}
