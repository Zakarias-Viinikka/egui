use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const START_DELAY: Duration = Duration::from_secs(5);
pub const CHECK_INTERVAL: Duration = Duration::from_secs(5);
pub const TRIGGER_PERCENT: f64 = 90.0;

pub fn used_percent() -> Result<f64, String> {
    let text = std::fs::read_to_string("/proc/meminfo").map_err(|e| {
        format!("err_msg: read /proc/meminfo failed: {e} | method: used_percent | file: app_core/src/ram_monitor.rs")
    })?;
    let mut total: Option<u64> = None;
    let mut available: Option<u64> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = rest.split_whitespace().next().and_then(|v| v.parse().ok());
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available = rest.split_whitespace().next().and_then(|v| v.parse().ok());
        }
    }
    let total = total.ok_or_else(|| {
        String::from("err_msg: MemTotal missing or unparsable | method: used_percent | file: app_core/src/ram_monitor.rs")
    })?;
    let available = available.ok_or_else(|| {
        String::from("err_msg: MemAvailable missing or unparsable | method: used_percent | file: app_core/src/ram_monitor.rs")
    })?;
    if total == 0 {
        return Err(String::from(
            "err_msg: MemTotal is 0 | method: used_percent | file: app_core/src/ram_monitor.rs",
        ));
    }
    let used = total - available.min(total);
    Ok(used as f64 / total as f64 * 100.0)
}

fn sleep_unless_stopped(stop: &AtomicBool, total: Duration) -> bool {
    let step = Duration::from_millis(100);
    let mut waited = Duration::ZERO;
    while waited < total {
        if stop.load(Ordering::SeqCst) {
            return true;
        }
        std::thread::sleep(step);
        waited += step;
    }
    stop.load(Ordering::SeqCst)
}

pub fn spawn<F>(stop: Arc<AtomicBool>, on_trip: F)
where
    F: FnOnce() + Send + 'static,
{
    std::thread::spawn(move || {
        if sleep_unless_stopped(&stop, START_DELAY) {
            return;
        }
        loop {
            let pct = match used_percent() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    return;
                }
            };
            if pct >= TRIGGER_PERCENT {
                if !stop.load(Ordering::SeqCst) {
                    on_trip();
                }
                return;
            }
            if sleep_unless_stopped(&stop, CHECK_INTERVAL) {
                return;
            }
        }
    });
}
