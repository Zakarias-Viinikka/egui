use crate::error::AppResult;
use crate::kdenlive::pull::{pull_from_kdenlive, PulledClip};
use crate::kdenlive::push::push_to_kdenlive;
use crate::kdenlive::talker::{check_status, KdenliveStatus};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub enum JobStatus {
    Running,
    Done,
    Failed(String),
}

pub struct Job {
    pub status: Arc<Mutex<JobStatus>>,
}

impl Job {
    pub fn is_running(&self) -> bool {
        matches!(*self.status.lock().unwrap(), JobStatus::Running)
    }
    pub fn current(&self) -> JobStatus {
        self.status.lock().map(|g| g.clone()).unwrap_or(JobStatus::Failed("lock poisoned".into()))
    }
}

pub struct KdenliveState {
    pub pulled: Option<PulledClip>,
    pub status: KdenliveStatus,
    last_status_check: Option<Instant>,
    last_status_was_multiple: bool,
    pub pull_job: Option<Job>,
    pull_slot: Option<Arc<Mutex<Option<AppResult<PulledClip>>>>>,
    pub push_job: Option<Job>,
    push_slot: Option<Arc<Mutex<Option<AppResult<String>>>>>,
}

impl KdenliveState {
    pub fn new() -> Self {
        Self {
            pulled: None,
            status: KdenliveStatus::Checking,
            last_status_check: None,
            last_status_was_multiple: false,
            pull_job: None,
            pull_slot: None,
            push_job: None,
            push_slot: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.status, KdenliveStatus::Connected { .. })
    }

    /// Refreshes Kdenlive status if >2s since last check.
    /// Returns Some(msg) only on the transition into Multiple, so caller can log once.
    pub fn maybe_refresh_status(&mut self) -> Option<String> {
        let due = self.last_status_check.map_or(true, |t| t.elapsed() >= Duration::from_secs(2));
        if !due {
            return None;
        }
        self.status = check_status();
        self.last_status_check = Some(Instant::now());

        let is_multiple = matches!(self.status, KdenliveStatus::Multiple { .. });
        let just_became = is_multiple && !self.last_status_was_multiple;
        self.last_status_was_multiple = is_multiple;

        if just_became {
            if let KdenliveStatus::Multiple { count, names } = &self.status {
                let pids: Vec<&str> = names
                    .iter()
                    .map(|n| n.strip_prefix("org.kde.kdenlive-").unwrap_or(n.as_str()))
                    .collect();
                return Some(format!(
                    "{count} Kdenlive instances running (PIDs: {}); close all but one",
                    pids.join(", ")
                ));
            }
        }
        None
    }

    pub fn is_busy(&self) -> bool {
        self.pull_job.as_ref().is_some_and(|j| j.is_running())
            || self.push_job.as_ref().is_some_and(|j| j.is_running())
    }

    pub fn start_pull(&mut self, output_dir: PathBuf) {
        if self.is_busy() { return; }
        let status = Arc::new(Mutex::new(JobStatus::Running));
        let slot: Arc<Mutex<Option<AppResult<PulledClip>>>> = Arc::new(Mutex::new(None));

        let s = status.clone();
        let sl = slot.clone();
        std::thread::spawn(move || {
            let out = pull_from_kdenlive(&output_dir);
            match &out {
                Ok(_) => *s.lock().unwrap() = JobStatus::Done,
                Err(e) => *s.lock().unwrap() = JobStatus::Failed(e.to_string()),
            }
            *sl.lock().unwrap() = Some(out);
        });

        self.pull_job = Some(Job { status });
        self.pull_slot = Some(slot);
    }

    pub fn poll_pull(&mut self) -> Option<AppResult<PulledClip>> {
        let job = self.pull_job.as_ref()?;
        if job.is_running() { return None; }
        let slot = self.pull_slot.as_ref()?;
        let result = slot.lock().unwrap().take();
        if result.is_some() {
            self.pull_slot = None;
            self.pull_job = None;
        }
        result
    }

    pub fn start_push(&mut self, source: PathBuf) {
        if self.is_busy() { return; }
        let status = Arc::new(Mutex::new(JobStatus::Running));
        let slot: Arc<Mutex<Option<AppResult<String>>>> = Arc::new(Mutex::new(None));

        let s = status.clone();
        let sl = slot.clone();
        std::thread::spawn(move || {
            let out = push_to_kdenlive(&source);
            match &out {
                Ok(_) => *s.lock().unwrap() = JobStatus::Done,
                Err(e) => *s.lock().unwrap() = JobStatus::Failed(e.to_string()),
            }
            *sl.lock().unwrap() = Some(out);
        });

        self.push_job = Some(Job { status });
        self.push_slot = Some(slot);
    }

    pub fn poll_push(&mut self) -> Option<AppResult<String>> {
        let job = self.push_job.as_ref()?;
        if job.is_running() { return None; }
        let slot = self.push_slot.as_ref()?;
        let result = slot.lock().unwrap().take();
        if result.is_some() {
            self.push_slot = None;
            self.push_job = None;
        }
        result
    }
}
