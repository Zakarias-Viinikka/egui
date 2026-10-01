use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const WINDOW: Duration = Duration::from_secs(60);
pub const CAP_DURATION: Duration = Duration::from_secs(60);
pub const TOP_PROGRAMS: usize = 5;
pub const LIMIT_PERCENT: u64 = 70;
const HELPER: &str = "/usr/local/bin/anti_freeze_cap";

pub enum Decision {
    Normal,
    Capped,
    AlreadyCapped,
}

struct State {
    last_trip: Option<Instant>,
    hogs: Vec<String>,
    cap_until: Option<Instant>,
}

static STATE: Mutex<State> = Mutex::new(State {
    last_trip: None,
    hogs: Vec::new(),
    cap_until: None,
});

struct Proc {
    pid: u32,
    name: String,
    rss_bytes: u64,
}

fn our_uid() -> Result<u32, String> {
    let status = std::fs::read_to_string("/proc/self/status").map_err(|e| {
        format!("err_msg: read /proc/self/status failed: {e} | method: our_uid | file: app_core/src/ram_cap.rs")
    })?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            if let Some(first) = rest.split_whitespace().next() {
                return first.parse::<u32>().map_err(|e| {
                    format!("err_msg: parse uid failed: {e} | method: our_uid | file: app_core/src/ram_cap.rs")
                });
            }
        }
    }
    Err(String::from(
        "err_msg: no Uid line in /proc/self/status | method: our_uid | file: app_core/src/ram_cap.rs",
    ))
}

fn user_procs() -> Result<Vec<Proc>, String> {
    let uid = our_uid()?;
    let our_pid = std::process::id();
    let own_name = std::fs::read_to_string("/proc/self/comm").map_err(|e| {
        format!("err_msg: read /proc/self/comm failed: {e} | method: user_procs | file: app_core/src/ram_cap.rs")
    })?;
    let own_name = own_name.trim().to_string();
    let entries = std::fs::read_dir("/proc").map_err(|e| {
        format!("err_msg: read_dir /proc failed: {e} | method: user_procs | file: app_core/src/ram_cap.rs")
    })?;
    let mut out: Vec<Proc> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let pid: u32 = match entry.file_name().to_string_lossy().parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if pid == our_pid {
            continue;
        }
        let status = match std::fs::read_to_string(entry.path().join("status")) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut name = String::new();
        let mut proc_uid: Option<u32> = None;
        let mut rss_kb: u64 = 0;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Name:") {
                name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("Uid:") {
                proc_uid = rest.split_whitespace().next().and_then(|v| v.parse().ok());
            } else if let Some(rest) = line.strip_prefix("VmRSS:") {
                rss_kb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
            }
        }
        if name.is_empty() || proc_uid != Some(uid) || name == own_name {
            continue;
        }
        if crate::killing::NEVER_STOP.contains(&name.as_str()) {
            continue;
        }
        out.push(Proc {
            pid,
            name,
            rss_bytes: rss_kb * 1024,
        });
    }
    Ok(out)
}

fn top_names(procs: &[Proc]) -> Vec<String> {
    let mut totals: HashMap<&str, u64> = HashMap::new();
    for p in procs {
        *totals.entry(p.name.as_str()).or_insert(0) += p.rss_bytes;
    }
    let mut v: Vec<(&str, u64)> = totals.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v.into_iter()
        .take(TOP_PROGRAMS)
        .map(|(n, _)| n.to_string())
        .collect()
}

fn mem_available_bytes() -> Result<u64, String> {
    let text = std::fs::read_to_string("/proc/meminfo").map_err(|e| {
        format!("err_msg: read /proc/meminfo failed: {e} | method: mem_available_bytes | file: app_core/src/ram_cap.rs")
    })?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: u64 = rest
                .split_whitespace()
                .next()
                .ok_or_else(|| String::from("err_msg: MemAvailable empty | method: mem_available_bytes | file: app_core/src/ram_cap.rs"))?
                .parse()
                .map_err(|e| {
                    format!("err_msg: parse MemAvailable failed: {e} | method: mem_available_bytes | file: app_core/src/ram_cap.rs")
                })?;
            return Ok(kb * 1024);
        }
    }
    Err(String::from(
        "err_msg: no MemAvailable in /proc/meminfo | method: mem_available_bytes | file: app_core/src/ram_cap.rs",
    ))
}

fn run_helper(args: &[String]) -> Result<(), String> {
    let output = Command::new("sudo")
        .arg("-n")
        .arg(HELPER)
        .args(args)
        .output()
        .map_err(|e| {
            format!("err_msg: spawn sudo failed: {e} | method: run_helper | file: app_core/src/ram_cap.rs")
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "err_msg: helper {:?} failed: {stderr} | method: run_helper | file: app_core/src/ram_cap.rs",
            args.first()
        ));
    }
    Ok(())
}

fn lock_err<T>(e: std::sync::PoisonError<T>, method: &str) -> String {
    format!("err_msg: state lock poisoned: {e} | method: {method} | file: app_core/src/ram_cap.rs")
}

pub fn on_trip() -> Result<Decision, String> {
    let mut st = STATE.lock().map_err(|e| lock_err(e, "on_trip"))?;
    let now = Instant::now();

    if let Some(until) = st.cap_until {
        if now < until {
            return Ok(Decision::AlreadyCapped);
        }
    }

    let recent = match st.last_trip {
        Some(t) => now.duration_since(t) < WINDOW,
        None => false,
    };
    let procs = user_procs()?;

    if recent && !st.hogs.is_empty() {
        let pids: Vec<String> = procs
            .iter()
            .filter(|p| st.hogs.contains(&p.name))
            .map(|p| p.pid.to_string())
            .collect();
        if !pids.is_empty() {
            let limit = mem_available_bytes()? / 2;
            let mut args = vec![String::from("apply"), limit.to_string()];
            args.extend(pids);
            run_helper(&args)?;
            st.last_trip = Some(now);
            st.cap_until = Some(now + CAP_DURATION);
            return Ok(Decision::Capped);
        }
    }

    st.hogs = top_names(&procs);
    st.last_trip = Some(now);
    Ok(Decision::Normal)
}

pub fn end_cap() -> Result<(), String> {
    {
        let mut st = STATE.lock().map_err(|e| lock_err(e, "end_cap"))?;
        st.cap_until = None;
        st.last_trip = Some(Instant::now());
    }
    run_helper(&[String::from("release")])
}
