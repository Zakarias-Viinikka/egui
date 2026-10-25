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
    save_from_steps(steps, statuses.to_vec());
}

/// Save a record from raw steps. Used when a chain fails before a Chain
/// exists: the parse succeeded but play_forward refused it, so there's
/// nothing to stash for review, but a retry should still be possible.
pub fn save_from_steps(steps: Vec<Step>, statuses: Vec<StepStatus>) {
    let statuses = if statuses.len() == steps.len() {
        statuses
    } else {
        vec![StepStatus::Pending; steps.len()]
    };
    *cell().lock().unwrap() = Some(Arc::new(RetryRecord { steps, statuses }));
}

/// Drop the stored record. Used by tests.
pub fn clear() {
    *cell().lock().unwrap() = None;
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
    replacements: Option<&[Step]>,
) -> Result<Chain, String> {
    let steps: Vec<Step> = match (only, replacements) {
        // Bare retry, or bare retry with explicit step numbers. The
        // listed steps are the ones resubmitted, unchanged.
        (None, None) | (Some(_), None) => payload(rec, only),
        (None, Some(_)) => {
            return Err(
                "retry: replacements need step numbers before them".to_string()
            );
        }
        // Retry with replacement bodies. Resend the full unapplied
        // payload with the named steps substituted, so the reviewer can
        // fix a broken step and get the whole chain back in one go.
        (Some(indices), Some(repl)) => {
            if repl.len() != indices.len() {
                return Err(format!(
                    "retry: {} step number(s) but {} replacement step(s)",
                    indices.len(),
                    repl.len()
                ));
            }
            let mut repl_by_index: std::collections::HashMap<usize, &Step> =
                std::collections::HashMap::new();
            for (i, step) in indices.iter().zip(repl.iter()) {
                if rec.statuses.get(*i) == Some(&StepStatus::Applied) {
                    return Err(format!(
                        "retry: step {} was already applied; can't replace it",
                        i + 1
                    ));
                }
                if repl_by_index.insert(*i, step).is_some() {
                    return Err(format!(
                        "retry: step {} is named twice",
                        i + 1
                    ));
                }
            }
            let mut out: Vec<Step> = Vec::new();
            for (i, step) in rec.steps.iter().enumerate() {
                if rec.statuses.get(i) == Some(&StepStatus::Applied) {
                    continue;
                }
                match repl_by_index.remove(&i) {
                    Some(new_step) => out.push(new_step.clone()),
                    None => out.push(step.clone()),
                }
            }
            if !repl_by_index.is_empty() {
                let mut names: Vec<String> =
                    repl_by_index.keys().map(|i| (i + 1).to_string()).collect();
                names.sort();
                return Err(format!(
                    "retry: step(s) {} not in the retry payload",
                    names.join(", ")
                ));
            }
            out
        }
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Verb;
    use std::fs;

    fn tmp() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("zap_retry_test_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    fn make_step(verb: Verb, path: &str) -> Step {
        Step {
            verb,
            path: path.to_string(),
            dest: None,
            old_text: String::new(),
            new_text: String::new(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    fn replace_step(path: &std::path::Path, old: &str, new: &str) -> Step {
        Step {
            verb: Verb::Replace,
            path: path.display().to_string(),
            dest: None,
            old_text: old.to_string(),
            new_text: new.to_string(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    #[test]
    fn payload_skips_applied() {
        let rec = RetryRecord {
            steps: vec![
                make_step(Verb::Create, "/tmp/a"),
                make_step(Verb::Create, "/tmp/b"),
                make_step(Verb::Create, "/tmp/c"),
            ],
            statuses: vec![
                StepStatus::Applied,
                StepStatus::Failed,
                StepStatus::Marked,
            ],
        };
        let p = payload(&rec, None);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].path, "/tmp/b");
        assert_eq!(p[1].path, "/tmp/c");
    }

    #[test]
    fn payload_all_applied_is_empty() {
        let rec = RetryRecord {
            steps: vec![make_step(Verb::Create, "/tmp/a")],
            statuses: vec![StepStatus::Applied],
        };
        assert!(payload(&rec, None).is_empty());
    }

    #[test]
    fn payload_with_only_returns_exactly_those() {
        let rec = RetryRecord {
            steps: vec![
                make_step(Verb::Create, "/tmp/a"),
                make_step(Verb::Create, "/tmp/b"),
                make_step(Verb::Create, "/tmp/c"),
            ],
            statuses: vec![StepStatus::Applied; 3],
        };
        let p = payload(&rec, Some(&[0, 2]));
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].path, "/tmp/a");
        assert_eq!(p[1].path, "/tmp/c");
    }

    #[test]
    fn payload_with_out_of_range_only_skips_them() {
        let rec = RetryRecord {
            steps: vec![make_step(Verb::Create, "/tmp/a")],
            statuses: vec![StepStatus::Failed],
        };
        let p = payload(&rec, Some(&[0, 5]));
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].path, "/tmp/a");
    }

    #[test]
    fn build_retry_chain_errors_on_empty_payload() {
        let rec = RetryRecord {
            steps: vec![],
            statuses: vec![],
        };
        let e = build_retry_chain(&rec, None, None).expect_err("empty");
        assert!(e.contains("nothing to retry"), "got: {e}");
    }

    #[test]
    fn build_retry_chain_errors_on_all_applied() {
        let rec = RetryRecord {
            steps: vec![make_step(Verb::Create, "/tmp/a")],
            statuses: vec![StepStatus::Applied],
        };
        let e = build_retry_chain(&rec, None, None).expect_err("all applied");
        assert!(e.contains("nothing to retry"), "got: {e}");
    }

    #[test]
    fn build_retry_chain_errors_on_run_step() {
        let mut cargo = make_step(Verb::Cargo, "/tmp/proj");
        cargo.args = vec!["test".to_string()];
        cargo.cwd = Some("/tmp/proj".to_string());
        let rec = RetryRecord {
            steps: vec![cargo],
            statuses: vec![StepStatus::Failed],
        };
        let e = build_retry_chain(&rec, None, None).expect_err("run");
        assert!(e.contains("run"), "got: {e}");
    }

    #[test]
    fn build_retry_chain_rebuilds_from_payload() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let rec = RetryRecord {
            steps: vec![replace_step(&file, "hello", "world")],
            statuses: vec![StepStatus::Failed],
        };
        let chain = build_retry_chain(&rec, None, None).expect("build");
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.steps[0].after, "world\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_retry_chain_only_selects_named_steps() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "one\n").expect("write");
        fs::write(&b, "two\n").expect("write");
        let rec = RetryRecord {
            steps: vec![
                replace_step(&a, "one", "ONE"),
                replace_step(&b, "two", "TWO"),
            ],
            statuses: vec![StepStatus::Failed, StepStatus::Failed],
        };
        let chain = build_retry_chain(&rec, Some(&[1]), None).expect("build");
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.steps[0].after, "TWO\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn global_record_saves_and_clears() {
        clear();
        assert!(current().is_none());

        save_from_steps(
            vec![
                make_step(Verb::Create, "/tmp/a"),
                make_step(Verb::Create, "/tmp/b"),
            ],
            vec![StepStatus::Failed],
        );
        let got = current().expect("record");
        assert_eq!(got.steps.len(), 2);
        assert_eq!(got.statuses.len(), 2);
        assert_eq!(got.statuses[0], StepStatus::Failed);
        assert_eq!(got.statuses[1], StepStatus::Pending);

        save_from_steps(
            vec![make_step(Verb::Create, "/tmp/a")],
            vec![StepStatus::Marked],
        );
        let got = current().expect("record");
        assert_eq!(got.statuses, vec![StepStatus::Marked]);

        clear();
        assert!(current().is_none());
    }

    #[test]
    fn build_retry_chain_uses_replacements() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let rec = RetryRecord {
            steps: vec![replace_step(&file, "wrong", "wrong")],
            statuses: vec![StepStatus::Failed],
        };
        let new_step = replace_step(&file, "hello", "goodbye");
        let chain = build_retry_chain(&rec, Some(&[0]), Some(&[new_step]))
            .expect("build");
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.steps[0].after, "goodbye\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_retry_chain_rejects_mismatched_replacements() {
        let rec = RetryRecord {
            steps: vec![make_step(Verb::Create, "/tmp/a")],
            statuses: vec![StepStatus::Failed],
        };
        let err = build_retry_chain(
            &rec,
            Some(&[0]),
            Some(&[
                make_step(Verb::Create, "/tmp/a"),
                make_step(Verb::Create, "/tmp/b"),
            ]),
        )
        .expect_err("mismatch");
        assert!(err.contains("step number"), "got: {err}");
    }

    #[test]
    fn build_retry_chain_with_replacements_resends_the_whole_payload() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        let c = dir.join("c.txt");
        fs::write(&a, "one\n").expect("write");
        fs::write(&b, "two\n").expect("write");
        fs::write(&c, "three\n").expect("write");
        let rec = RetryRecord {
            steps: vec![
                replace_step(&a, "one", "ONE"),
                replace_step(&b, "two", "TWO"),
                replace_step(&c, "three", "THREE"),
            ],
            statuses: vec![
                StepStatus::Failed,
                StepStatus::Failed,
                StepStatus::Failed,
            ],
        };
        let fixed = replace_step(&b, "two\n", "two-fixed");
        let chain = build_retry_chain(&rec, Some(&[1]), Some(&[fixed]))
            .expect("build");
        assert_eq!(chain.steps.len(), 3, "all steps should be retried");
        assert_eq!(chain.steps[0].after, "ONE\n");
        assert_eq!(chain.steps[1].after, "two-fixed");
        assert_eq!(chain.steps[2].after, "THREE\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_retry_chain_with_replacements_skips_applied_steps() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "one\n").expect("write");
        fs::write(&b, "two\n").expect("write");
        let rec = RetryRecord {
            steps: vec![
                replace_step(&a, "one", "ONE"),
                replace_step(&b, "two", "TWO"),
            ],
            statuses: vec![StepStatus::Applied, StepStatus::Failed],
        };
        let fixed = replace_step(&b, "two\n", "two-fixed");
        let chain = build_retry_chain(&rec, Some(&[1]), Some(&[fixed]))
            .expect("build");
        assert_eq!(chain.steps.len(), 1, "applied step is not resent");
        assert_eq!(chain.steps[0].after, "two-fixed");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_retry_chain_refuses_to_replace_an_applied_step() {
        let rec = RetryRecord {
            steps: vec![make_step(Verb::Create, "/tmp/a")],
            statuses: vec![StepStatus::Applied],
        };
        let err = build_retry_chain(
            &rec,
            Some(&[0]),
            Some(&[make_step(Verb::Create, "/tmp/a")]),
        )
        .expect_err("should fail");
        assert!(err.contains("already applied"), "got: {err}");
    }
}
