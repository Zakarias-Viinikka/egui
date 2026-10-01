use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Default)]
pub struct TmpStats {
    pub ram_mb: u32,
    pub cpu_pct: u32,
    pub gpu_pct: u32,
}

pub fn spawn_stats_watcher<F>(shared: Arc<Mutex<TmpStats>>, on_update: F)
where
    F: Fn() + Send + 'static,
{
    std::thread::spawn(move || {
        let mut prev_total: u64 = 0;
        let mut prev_idle: u64 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));

            let ram_mb = match read_used_mb() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    0
                }
            };

            let (total, idle) = match read_cpu_ticks() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    (prev_total, prev_idle)
                }
            };
            let cpu_pct = if prev_total == 0 || total <= prev_total {
                0
            } else {
                let dt = total - prev_total;
                let di = idle.saturating_sub(prev_idle);
                let busy = dt.saturating_sub(di);
                ((busy * 100) / dt) as u32
            };
            prev_total = total;
            prev_idle = idle;

            let gpu_pct = read_gpu_pct().unwrap_or(0);

            match shared.lock() {
                Ok(mut g) => {
                    g.ram_mb = ram_mb;
                    g.cpu_pct = cpu_pct;
                    g.gpu_pct = gpu_pct;
                }
                Err(e) => {
                    eprintln!("tmp_tester: lock poisoned: {e}");
                    return;
                }
            }
            on_update();
        }
    });
}

fn read_used_mb() -> Result<u32, String> {
    let contents = std::fs::read_to_string("/proc/meminfo").map_err(|e| {
        format!("err_msg: read /proc/meminfo failed: {e} | method: read_used_mb | file: app_core/src/reading_ram/tmp_tester.rs")
    })?;
    let mut total: u64 = 0;
    let mut available: u64 = 0;
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            if let Some(first) = rest.split_whitespace().next() {
                total = first.parse().unwrap_or(0);
            }
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            if let Some(first) = rest.split_whitespace().next() {
                available = first.parse().unwrap_or(0);
            }
        }
    }
    let used_kb = total.saturating_sub(available);
    Ok((used_kb / 1024) as u32)
}

fn read_cpu_ticks() -> Result<(u64, u64), String> {
    let contents = std::fs::read_to_string("/proc/stat").map_err(|e| {
        format!("err_msg: read /proc/stat failed: {e} | method: read_cpu_ticks | file: app_core/src/reading_ram/tmp_tester.rs")
    })?;
    let first = match contents.lines().next() {
        Some(l) => l,
        None => {
            return Err(String::from(
                "err_msg: /proc/stat empty | method: read_cpu_ticks | file: app_core/src/reading_ram/tmp_tester.rs",
            ))
        }
    };
    if !first.starts_with("cpu ") {
        return Err(String::from(
            "err_msg: /proc/stat first line not cpu | method: read_cpu_ticks | file: app_core/src/reading_ram/tmp_tester.rs",
        ));
    }
    let nums: Vec<u64> = first
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    if nums.len() < 4 {
        return Err(String::from(
            "err_msg: /proc/stat cpu line too short | method: read_cpu_ticks | file: app_core/src/reading_ram/tmp_tester.rs",
        ));
    }
    let total: u64 = nums.iter().sum();
    // user nice system idle iowait irq softirq steal
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    Ok((total, idle))
}

fn read_gpu_pct() -> Result<u32, String> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|e| {
            format!("err_msg: nvidia-smi spawn failed: {e} | method: read_gpu_pct | file: app_core/src/reading_ram/tmp_tester.rs")
        })?;
    if !output.status.success() {
        return Err(format!(
            "err_msg: nvidia-smi exited {} | method: read_gpu_pct | file: app_core/src/reading_ram/tmp_tester.rs",
            output.status
        ));
    }
    let s = String::from_utf8_lossy(&output.stdout);
    let first = s.lines().next().unwrap_or("").trim();
    first.parse::<u32>().map_err(|e| {
        format!("err_msg: parse nvidia-smi output {first:?} failed: {e} | method: read_gpu_pct | file: app_core/src/reading_ram/tmp_tester.rs")
    })
}
