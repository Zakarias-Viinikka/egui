//! The last finished review, with per-step status, so a `retry` request
//! can rebuild a fresh chain from only the steps that didn't apply.
//!
//! Written by the UI thread when a chain finishes (Confirm, Deny All, or
//! a writer failure). Read by `handle_retry` in route_requests.rs.

use crate::chain::{Chain, Step, StepStatus};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

pub struct RetryRecord {
    pub steps: Vec<Step>,
    pub statuses: Vec<StepStatus>,
}

static LAST: OnceLock<Mutex<Option<Arc<RetryRecord>>>> = OnceLock::new();

fn cell() -> &'static Mutex<Option<Arc<RetryRecord>>> {
    LAST.get_or_init(|| Mutex::new(None))
}

pub fn save_from_chain(chain: &Chain, statuses: &[StepStatus]) {
    let steps: Vec<Step> = chain.steps.iter().map(|s| s.step.clone()).collect();
    let statuses: Vec<StepStatus> = if statuses.len() == steps.len() {
        statuses.to_vec()
    } else {
        vec![StepStatus::Pending; steps.len()]
    };
    *cell().lock().unwrap() = Some(Arc::new(RetryRecord { steps, statuses }));
}

pub fn current() -> Option<Arc<RetryRecord>> {
    cell().lock().unwrap().clone()
}

/// Steps that didn't apply. If `only` is set, take exactly those indices
/// (0-based) from the original chain and ignore the status filter.
pub fn payload(rec: &RetryRecord, only: Option<&[usize]>) -> Vec<Step> {
    match only {
        Some(idx) => idx
            .iter()
            .filter_map(|i| rec.steps.get(*i).cloned())
            .collect(),
        None => rec
            .steps
            .iter()
            .enumerate()
            .filter(|(i, _)| rec.statuses.get(*i) != Some(&StepStatus::Applied))
            .map(|(_, s)| s.clone())
            .collect(),
    }
}

/// Build a fresh chain from the retry payload. Re-reads each file from
/// disk via `play_forward`, so the diff shown is against current state.
pub fn build_retry_chain(
    rec: &RetryRecord,
    only: Option<&[usize]>,
) -> Result<Chain, String> {
    let steps = payload(rec, only);
    if steps.is_empty() {
        return Err("nothing to retry".to_string());
    }
    if steps.iter().any(|s| s.verb.is_run()) {
        return Err("retry of a run step isn't supported; resend it as a chain".to_string());
    }
    // All chain write paths are absolute or ~; cwd is unused by resolve().
    let cwd = PathBuf::from("/");
    crate::chain::play_forward(steps, &cwd)
}
